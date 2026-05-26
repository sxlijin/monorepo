// Site handler for roundcolors.com. Invoked via a Lambda Function URL, so it
// receives an API Gateway v2 "payload format 2.0" event and must return a
// response shaped for that format.
const HTML = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>roundcolors.com</title>
    <style>
      :root { color-scheme: light dark; }
      body {
        margin: 0;
        min-height: 100vh;
        display: grid;
        place-items: center;
        font-family: system-ui, sans-serif;
        background: #0d0d12;
        color: #f5f5f7;
      }
      h1 { font-size: clamp(2rem, 8vw, 5rem); margin: 0; }
      p { opacity: 0.7; }
    </style>
  </head>
  <body>
    <main style="text-align: center">
      <h1>roundcolors.com</h1>
      <p>Coming soon.</p>
    </main>
  </body>
</html>
`;

export const handler = async () => ({
  statusCode: 200,
  headers: {
    'content-type': 'text/html; charset=utf-8',
    'cache-control': 'public, max-age=60',
  },
  body: HTML,
});
