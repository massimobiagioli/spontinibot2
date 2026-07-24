import pa11y from 'pa11y';
import { preview } from 'vite';

const PORT = 4173;
const ROUTES = ['/', '/dev'];

const server = await preview({ preview: { port: PORT, host: '127.0.0.1' } });
const baseUrl = `http://127.0.0.1:${PORT}`;

let hasErrors = false;

try {
  for (const route of ROUTES) {
    const url = `${baseUrl}${route}`;
    const result = await pa11y(url, {
      chromeLaunchConfig: { args: ['--no-sandbox'] },
    });
    const errors = result.issues.filter((issue) => issue.type === 'error');

    if (errors.length > 0) {
      hasErrors = true;
      console.error(`\n${url}: ${errors.length} error(s)`);
      for (const issue of errors) {
        console.error(
          `  - [${issue.code}] ${issue.message} (${issue.selector})`,
        );
      }
    } else {
      console.log(`${url}: 0 errors`);
    }
  }
} finally {
  await new Promise((resolve) => server.httpServer.close(resolve));
}

if (hasErrors) {
  process.exitCode = 1;
}
