# Frontend image (Next.js)
FROM node:20-bookworm-slim AS builder
WORKDIR /build
ARG NPM_REGISTRY=https://registry.npmmirror.com
COPY frontend/. ./frontend
RUN npm config set registry ${NPM_REGISTRY} \
    && npm i -g pnpm@9.0.0 \
    && pnpm config set registry ${NPM_REGISTRY} \
    && cd frontend \
    && pnpm install --no-frozen-lockfile \
    && pnpm build

FROM node:20-bookworm-slim AS runtime
WORKDIR /app/frontend
COPY --from=builder /build/frontend/.next ./.next
COPY --from=builder /build/frontend/public ./public
COPY --from=builder /build/frontend/package.json ./package.json
COPY --from=builder /build/frontend/node_modules ./node_modules
COPY --from=builder /build/frontend/next.config.* ./
ENV PORT=3000 HOST=0.0.0.0 \
    NEXT_PUBLIC_BACKEND_URL=http://backend:8080
EXPOSE 3000
CMD ["node", "node_modules/next/dist/bin/next", "start", "-p", "3000", "-H", "0.0.0.0"]


