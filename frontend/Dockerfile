# FROM oven/bun:1 AS base
# WORKDIR /app

# # Install dependencies into temp directory this will cache them for future builds.
# FROM base AS install
# RUN mkdir -p /temp/dev
# COPY package.json bun.lock /temp/dev/
# RUN cd /temp/dev && bun install --frozen-lockfile

# # Install with --production (exclude devDependencies).
# RUN mkdir -p /temp/prod
# COPY package.json bun.lock /temp/prod/
# RUN cd /temp/prod && bun install --frozen-lockfile --production

# # Copy node_modules from temp directory then copy all project files into the image.
# FROM base AS prerelease
# COPY --from=install /temp/dev/node_modules node_modules
# COPY . .

# # Test & build
# # ENV NODE_ENV=production
# ENV NODE_ENV=development
# #RUN bun test
# RUN bun run build

# # Copy dependencies and source code into final image.
# FROM base AS release
# COPY --from=install /temp/prod/node_modules node_modules
# COPY --from=prerelease /app/build ./build
# COPY --from=prerelease /app/package.json .

# # Run the app
# USER bun
# EXPOSE 3000/tcp

# # ENTRYPOINT [ "bun", "--bun", "run", "build/index.js" ]
# ENTRYPOINT [ "bun", "--bun", "run", "build/index.js" ]

FROM oven/bun:1 AS base
WORKDIR /app

# --- Stage 1: Install Dependencies ---
FROM base AS install
RUN mkdir -p /temp/dev
COPY package.json bun.lock /temp/dev/
RUN cd /temp/dev && bun install --frozen-lockfile

RUN mkdir -p /temp/prod
COPY package.json bun.lock /temp/prod/
RUN cd /temp/prod && bun install --frozen-lockfile --production

# --- Stage 2: Development Image ---
FROM base AS dev
COPY --from=install /temp/dev/node_modules node_modules
COPY . .
ENV NODE_ENV=development
# "bun run dev" usually defaults to port 5173
EXPOSE 5173 
# --host is required for Docker networking
CMD [ "bun", "run", "dev", "--host" ] 

# --- Stage 3: Build Source ---
FROM base AS prerelease
COPY --from=install /temp/dev/node_modules node_modules
COPY . .
ENV NODE_ENV=production
RUN bun run build

# --- Stage 4: Production Release ---
FROM base AS release
COPY --from=install /temp/prod/node_modules node_modules
COPY --from=prerelease /app/build ./build
COPY --from=prerelease /app/package.json .

USER bun
EXPOSE 3000/tcp
ENTRYPOINT [ "bun", "--bun", "run", "build/index.js" ]