import pino, { type Logger } from 'pino';
export const getLogger = (logLevel?: string): Logger<never> =>
  pino({
    level: logLevel || 'info',
    ...(process.env.LOG_FORMAT === 'json'
      ? {}
      : {
          transport: {
            target: process.env.LOG_FORMAT || 'pino-pretty',
          },
        }),
  });
