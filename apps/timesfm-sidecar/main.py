import os
import logging
import math
import numpy as np
from fastapi import FastAPI, Depends, HTTPException, Security
from fastapi.security.api_key import APIKeyHeader
from pydantic import BaseModel, ConfigDict, Field, field_validator
from contextlib import asynccontextmanager
from typing import List, Optional

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("timesfm-sidecar")

# Auth
API_KEY = os.environ.get("TIMESFM_AUTH_TOKEN")
API_KEY_NAME = "Authorization"
api_key_header = APIKeyHeader(name=API_KEY_NAME, auto_error=False)

def get_api_key(api_key_header: str = Security(api_key_header)):
    # RT-3 FIX: Reject ALL requests when auth token is not configured.
    # Previous code had `if API_KEY and ...` which silently bypassed auth
    # when TIMESFM_AUTH_TOKEN was not set.
    if not API_KEY:
        raise HTTPException(
            status_code=500, detail="Server misconfiguration: TIMESFM_AUTH_TOKEN not set."
        )
    if not api_key_header:
        raise HTTPException(
            status_code=401, detail="Missing Authorization header."
        )
    if api_key_header != f"Bearer {API_KEY}" and api_key_header != API_KEY:
        raise HTTPException(
            status_code=403, detail="Could not validate credentials"
        )
    return api_key_header

# Model specific context
class ModelContext:
    def __init__(self):
        self.tfm = None

model_ctx = ModelContext()

@asynccontextmanager
async def lifespan(app: FastAPI):
    # Load model on startup
    try:
        import timesfm
        try:
            # First try the newer 2.5 torch version
            tfm = timesfm.TimesFM_2p5_200M_torch.from_pretrained("google/timesfm-2.5-200m-pytorch")
            tfm.compile(
                timesfm.ForecastConfig(
                    max_context=1024,
                    max_horizon=256,
                    normalize_inputs=True,
                    use_continuous_quantile_head=True,
                    force_flip_invariance=True,
                    infer_is_positive=True,
                    fix_quantile_crossing=True,
                )
            )
            model_ctx.tfm = tfm
            logger.info("TimesFM 2.5 loaded successfully.")
        except AttributeError:
            # Fallback to older 1.0 or 2.0 version loading if 2.5 breaks
            tfm = timesfm.TimesFm(
                context_len=512,
                horizon_len=256,
                input_patch_len=32,
                output_patch_len=128,
                num_layers=20,
                model_dims=1280,
                backend="cpu",
            )
            tfm.load_from_checkpoint(repo_id="google/timesfm-1.0-200m")
            model_ctx.tfm = tfm
            logger.info("TimesFM 1.0 fallback loaded successfully.")
    except Exception as e:
        logger.error(f"Failed to load TimesFM model: {e}")
        pass
        
    yield
    # Cleanup on shutdown
    model_ctx.tfm = None

app = FastAPI(lifespan=lifespan)

# RT-2 FIX: Strict input validation with size limits to prevent OOM/DoS.
# RT-4 FIX: Reject NaN/Infinity at the API boundary.
MAX_SERIES_COUNT = 10
MAX_ELEMENTS_PER_SERIES = 2048

class ForecastRequest(BaseModel):
    series: List[List[float]] = Field(..., max_length=MAX_SERIES_COUNT)
    horizon: int = Field(..., gt=0, le=256)
    context_length: Optional[int] = Field(512, le=1024)
    quantiles: Optional[bool] = True

    @field_validator('series')
    @classmethod
    def validate_series(cls, v):
        if not v:
            raise ValueError('series must not be empty')
        for i, s in enumerate(v):
            if len(s) > MAX_ELEMENTS_PER_SERIES:
                raise ValueError(f'series[{i}] has {len(s)} elements (max {MAX_ELEMENTS_PER_SERIES})')
            if len(s) == 0:
                raise ValueError(f'series[{i}] is empty')
            # RT-4 FIX: Reject NaN and Infinity
            for j, val in enumerate(s):
                if math.isnan(val) or math.isinf(val):
                    raise ValueError(f'series[{i}][{j}] is NaN or Infinity')
        return v

class ForecastResponse(BaseModel):
    point_forecast: List[List[float]]
    quantile_forecast: Optional[List[List[List[float]]]] = None
    model_version: str

@app.get("/health")
async def health():
    if model_ctx.tfm is None:
        return {"status": "starting", "model_loaded": False}
    return {"status": "ok", "model_loaded": True}

@app.post("/forecast", response_model=ForecastResponse)
async def forecast(req: ForecastRequest, _auth=Depends(get_api_key)):
    if model_ctx.tfm is None:
        raise HTTPException(status_code=503, detail="Model is not loaded.")
    
    try:
        # Prepare inputs
        inputs = [np.array(s, dtype=np.float32) for s in req.series]
        
        if hasattr(model_ctx.tfm, 'forecast'):
            point_forecast, quantile_forecast = model_ctx.tfm.forecast(
                horizon=req.horizon,
                inputs=inputs,
            )
            
            p_res = point_forecast.tolist()
            q_res = quantile_forecast.tolist() if req.quantiles else None
            
            return ForecastResponse(
                point_forecast=p_res,
                quantile_forecast=q_res,
                model_version="timesfm-2.5-200m",
            )
        else:
             raise HTTPException(status_code=500, detail="Unsupported model object version (no forecast method).")
            
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error during forecast: {e}", exc_info=True)
        # RT-6 FIX: Do not expose raw exception details to the client.
        # Log the full traceback server-side, return only a generic message.
        raise HTTPException(status_code=500, detail="Internal forecast engine error. Check server logs.")
