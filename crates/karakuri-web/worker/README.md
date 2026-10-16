# Karakuri Share Microservice (Cloudflare Worker)

Public storage endpoint for Karakuri Web when `.kbset` bundles exceed the Twitter / Deflate URL hash limit (~3,800 characters).

## Architecture & Storage Policy

- **Path**: `POST /set`
  - Accepts raw `.kbset` NDJSON bundle (max 512 KB).
  - Validates `Origin` against allowed origin (`toyoshim-i.github.io`).
  - Verifies Cloudflare Turnstile token (`CF-Turnstile-Token` header) with Cloudflare Siteverify API.
  - Generates content-addressed ID (`sha256[:16]`) for deduplication.
  - Stores into Cloudflare KV (`SETS`) **permanently** without expiration TTL.
  - Returns JSON `{ id: "...", rawUrl: "https://.../set/..." }`.

- **Path**: `GET /set/:id`
  - Retrieves raw `.kbset` bundle by 16-character hex ID.
  - Cached for 24h at Cloudflare Edge.

## Deployment & Setup

### 1. Prerequisites
Install Wrangler CLI:
```bash
npm install -g wrangler
# or run via npx
```

### 2. KV Namespace
Ensure a KV namespace is created:
```bash
npx wrangler kv:namespace create "KARAKURI_SETS"
```
Update `wrangler.toml` with the generated KV namespace ID.

### 3. Turnstile Secret
Store the secret key generated from Cloudflare Dashboard > Turnstile:
```bash
npx wrangler secret put TURNSTILE_SECRET
```

### 4. Deploy
```bash
npx wrangler deploy
```

### 5. Local Simulation
```bash
npx wrangler dev
```
