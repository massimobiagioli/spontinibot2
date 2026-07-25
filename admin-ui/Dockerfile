FROM node:22-alpine AS build

RUN apk add --no-cache nginx chromium nss freetype freetype-dev harfbuzz ca-certificates ttf-freefont

# Puppeteer (used by pa11y — make lint) has no musl-compatible bundled
# Chromium; use the Alpine system package instead.
ENV PUPPETEER_SKIP_DOWNLOAD=true \
    PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium-browser

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build

RUN mkdir -p /usr/share/nginx/html /var/lib/nginx/tmp /var/log/nginx \
    && cp -r dist/* /usr/share/nginx/html/

COPY nginx.conf /etc/nginx/nginx.conf

RUN chown -R node:node /app /usr/share/nginx/html /var/lib/nginx /var/log/nginx
USER node

EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]

# Production runtime: static dist/ + nginx only. Base image is plain alpine,
# not node:22-alpine — the runtime needs no Node.js/npm at all (those, plus
# Chromium/puppeteer, exist solely for `docker compose run --rm admin-ui npm
# run test|lint|a11y`, a dev-time concern), and node:22-alpine's own bundled
# npm dependency tree carries known CVEs that plain alpine doesn't.
FROM alpine:3.24 AS runtime

RUN apk add --no-cache nginx \
    && addgroup -g 10001 spontini \
    && adduser -D -u 10001 -G spontini spontini \
    && mkdir -p /usr/share/nginx/html /var/lib/nginx/tmp /var/log/nginx

COPY --from=build --chown=spontini:spontini /usr/share/nginx/html /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf

RUN chown -R spontini:spontini /var/lib/nginx /var/log/nginx
USER spontini

EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]
