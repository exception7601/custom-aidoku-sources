FROM oven/bun:1 AS base
WORKDIR /app

FROM base AS install
RUN mkdir -p /temp/dev
COPY package.json bun.lockb* /temp/dev/
RUN cd /temp/dev && bun install --frozen-lockfile

FROM base AS prerelease
COPY --from=install /temp/dev/node_modules node_modules
COPY . .

ENV NODE_ENV=production
RUN bun run type-check
RUN bun run check

FROM base AS release
COPY --from=prerelease /app/node_modules node_modules
COPY --from=prerelease /app/src src
COPY --from=prerelease /app/tsconfig.json .

EXPOSE 4000
ENV NODE_ENV=production
ENV PORT=4000

CMD ["bun", "run", "src/index.ts"]
