# Stage 1: Build the SPA
FROM node:20-alpine AS builder

WORKDIR /app

# Install dependencies securely.
# We copy .npmrc first which contains `ignore-scripts=true`.
# This is a critical supply-chain mitigation. We then manually
# rebuild only the packages that legitimately require native binaries.
COPY package.json package-lock.json .npmrc ./
RUN npm ci

# Rebuild esbuild for Vite
RUN npm rebuild esbuild

# Now copy the rest (including source code, etc.)
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
