FROM node:22-alpine

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

RUN mkdir -p /usr/share/nginx/html \
    && cp -r dist/* /usr/share/nginx/html/

COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
