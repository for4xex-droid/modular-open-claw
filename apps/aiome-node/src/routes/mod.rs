use axum::routing::get;
use axum::Router;

pub mod agent_card;

pub fn well_known_routes() -> Router {
    Router::new().route("/agent.json", get(agent_card::get_agent_card))
}
