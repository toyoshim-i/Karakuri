/**
 * Karakuri Public Storage Microservice (Cloudflare Worker)
 *
 * Provides a lightweight, content-addressed public storage endpoint for
 * sharing .kbset NDJSON bundles that exceed the inline Deflate URL hash limit (~3,800 chars).
 *
 * Storage Policy:
 * - Content-addressed by SHA-256 (16-char hex prefix) for automatic deduplication.
 * - Indefinite retention (permanent storage in Cloudflare KV without expiration TTL).
 * - Maximum payload size: 512 KB.
 */

// Maximum allowed payload size for a .kbset bundle (512 KB)
const MAX_PAYLOAD_BYTES = 512 * 1024;

const ALLOWED_ORIGINS = [
  'https://toyoshim-i.github.io',
];

function isOriginAllowed(origin) {
  if (!origin) return false;
  return ALLOWED_ORIGINS.some(allowed => origin.startsWith(allowed));
}

function getCorsHeaders(request) {
  const origin = request.headers.get('Origin') || '';
  const allowOrigin = isOriginAllowed(origin) ? origin : 'https://toyoshim-i.github.io';
  return {
    'Access-Control-Allow-Origin': allowOrigin,
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, CF-Turnstile-Token',
    'Vary': 'Origin',
  };
}

async function computeSha256Short(text) {
  const msgUint8 = new TextEncoder().encode(text);
  const hashBuffer = await crypto.subtle.digest('SHA-256', msgUint8);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
  return hex.slice(0, 16); // 16-character content hash
}

export default {
  async fetch(request, env) {
    const corsHeaders = getCorsHeaders(request);

    // Handle preflight OPTIONS request
    if (request.method === 'OPTIONS') {
      return new Response(null, { headers: corsHeaders });
    }

    const url = new URL(request.url);

    try {
      // POST /set - Store a Set / shader bundle in KV
      if (url.pathname === '/set' || url.pathname === '/set/') {
        if (request.method !== 'POST') {
          return new Response(JSON.stringify({ error: 'Method Not Allowed. Use POST to upload sets.' }), {
            status: 405,
            headers: { ...corsHeaders, 'Content-Type': 'application/json', 'Allow': 'POST, OPTIONS' },
          });
        }

        // 1. Origin verification
        const origin = request.headers.get('Origin') || '';
        if (!isOriginAllowed(origin)) {
          return new Response(JSON.stringify({ error: 'Forbidden origin' }), {
            status: 403,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        // 2. Cloudflare Turnstile token validation
        const turnstileToken = request.headers.get('CF-Turnstile-Token') || '';
        if (!turnstileToken) {
          return new Response(JSON.stringify({ error: 'Missing Turnstile token' }), {
            status: 403,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        if (env.TURNSTILE_SECRET) {
          const formData = new FormData();
          formData.append('secret', env.TURNSTILE_SECRET);
          formData.append('response', turnstileToken);
          formData.append('remoteip', request.headers.get('CF-Connecting-IP') || '');

          try {
            const verifyRes = await fetch('https://challenges.cloudflare.com/turnstile/v0/siteverify', {
              method: 'POST',
              body: formData,
            });
            const outcome = await verifyRes.json();
            if (!outcome.success) {
              return new Response(JSON.stringify({
                error: 'Turnstile verification failed',
                details: outcome['error-codes'],
              }), {
                status: 403,
                headers: { ...corsHeaders, 'Content-Type': 'application/json' },
              });
            }
          } catch (verifyErr) {
            return new Response(JSON.stringify({
              error: 'Turnstile verification network error',
              message: String(verifyErr),
            }), {
              status: 502,
              headers: { ...corsHeaders, 'Content-Type': 'application/json' },
            });
          }
        }

        // 3. Payload size check (512 KB)
        const contentLength = parseInt(request.headers.get('Content-Length') || '0', 10);
        if (contentLength > MAX_PAYLOAD_BYTES) {
          return new Response(JSON.stringify({ error: 'Payload too large (max 512KB)' }), {
            status: 413,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        const body = await request.text();
        if (!body || body.length > MAX_PAYLOAD_BYTES) {
          return new Response(JSON.stringify({ error: 'Payload too large or empty (max 512KB)' }), {
            status: 413,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        // 4. Content format check
        const firstLine = body.split('\n')[0].trim();
        const isValidKarakuri = firstLine.includes('"t":"set"') || firstLine.includes('"t":"header"');
        if (!isValidKarakuri) {
          return new Response(JSON.stringify({ error: 'Invalid Karakuri Set format: expected NDJSON header' }), {
            status: 400,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        // 5. Content-addressed ID (deduplicates exact same data and prevents storage exhaustion)
        const id = await computeSha256Short(body);

        // 6. Save to KV indefinitely (no expiration TTL = permanent storage)
        const kv = env.SETS || env.KARAKURI_SETS;
        if (!kv) {
          return new Response(JSON.stringify({ error: 'KV namespace binding (SETS) is not configured in Worker' }), {
            status: 500,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        await kv.put(id, body);
        const rawUrl = `${url.origin}/set/${id}`;

        return new Response(JSON.stringify({ id, rawUrl }), {
          status: 200,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        });
      }

      // GET /set/:id - Retrieve raw Set from KV
      if (url.pathname.startsWith('/set/')) {
        if (request.method !== 'GET') {
          return new Response(JSON.stringify({ error: 'Method Not Allowed. Use GET to retrieve sets.' }), {
            status: 405,
            headers: { ...corsHeaders, 'Content-Type': 'application/json', 'Allow': 'GET, OPTIONS' },
          });
        }

        const id = url.pathname.slice(5).trim();
        if (!/^[a-f0-9]{8,32}$/i.test(id)) {
          return new Response(JSON.stringify({ error: 'Invalid Set ID format' }), {
            status: 400,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        const kv = env.SETS || env.KARAKURI_SETS;
        if (!kv) {
          return new Response(JSON.stringify({ error: 'KV namespace binding (SETS) is not configured in Worker' }), {
            status: 500,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        const content = await kv.get(id);
        if (!content) {
          return new Response(JSON.stringify({ error: 'Set not found' }), {
            status: 404,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        return new Response(content, {
          status: 200,
          headers: {
            ...corsHeaders,
            'Content-Type': 'text/plain; charset=utf-8',
            'Cache-Control': 'public, max-age=86400',
          },
        });
      }

      // Fallback service root
      return new Response('Karakuri Share Service (Cloudflare Worker)', {
        status: 200,
        headers: { ...corsHeaders, 'Content-Type': 'text/plain; charset=utf-8' },
      });
    } catch (err) {
      return new Response(JSON.stringify({ error: 'Internal Worker Exception', details: String(err) }), {
        status: 500,
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      });
    }
  },
};
