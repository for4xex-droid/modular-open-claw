# Stage 1: Build the SPA
FROM node:20-alpine AS builder

WORKDIR /app

# Install dependencies BEFORE copying .npmrc.
# The project's .npmrc sets ignore-scripts=true (supply-chain attack mitigation),
# but esbuild/rollup require postinstall scripts to download platform-specific
# binaries. By copying only package files first, npm ci runs with scripts enabled.
COPY package.json package-lock.json ./
RUN npm ci

# Now copy the rest (including .npmrc, source code, etc.)
COPY . .

# Build the application (tsc type-check + vite bundle)
RUN npm run build

# Stage 2: Serve with Nginx
FROM nginx:alpine

# Copy built assets to Nginx
COPY --from=builder /app/dist /usr/share/nginx/html

# Overwrite default nginx config with our SPA config
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -qO- http://localhost:80/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
