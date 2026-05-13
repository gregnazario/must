# Docker Recipes

must provides two Docker recipe types: `docker-build` and `docker-push`. Podman is used as a fallback if Docker is not available.

## Quick start

Project structure:

```
my-app/
├── Dockerfile
├── app.js
├── package.json
└── Mustfile.toml
```

**Dockerfile**

```dockerfile
FROM node:22-alpine AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
```

**app.js**

```js
const express = require("express");
const app = express();

app.get("/", (req, res) => {
  res.send("Hello from must!");
});

app.listen(3000);
```

**package.json**

```json
{
  "name": "my-app",
  "version": "1.0.0",
  "scripts": {
    "build": "echo 'no build step'",
    "start": "node app.js"
  },
  "dependencies": {
    "express": "^4.21.0"
  }
}
```

**Mustfile.toml**

```toml
[project]
name = "my-app"
version = "1.0.0"

[recipe.build]
type       = "docker-build"
image      = "myapp:latest"
dockerfile = "Dockerfile"
context    = "."
build_args = ["VERSION=1.0.0"]
```

Build the image:

```
$ must build
[build] docker build -t myapp:latest -f Dockerfile --build-arg VERSION=1.0.0 .
[build] => exporting to image
[build] => => writing image sha256:a1b2c3d4
[build] done (12.3s)
```

## Try it

<div id="playground-docker" data-must-playground="docker"></div>

## `docker-build` — Build an image

```toml
[recipe.build]
type       = "docker-build"
image      = "myapp:latest"
dockerfile = "Dockerfile"
context    = "."
build_args = ["VERSION=1.0"]
```

Runs: `docker build -t myapp:latest -f Dockerfile --build-arg VERSION=1.0 .`

Cache key includes: image, dockerfile, context, build_args, env vars.

## `docker-push` — Push an image

```toml
[recipe.push]
type  = "docker-push"
image = "myregistry/myapp:latest"
deps  = ["build"]
```

Runs: `docker push myregistry/myapp:latest`

Cache strategy: `never` (push should always execute).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `image` | string | all | Docker image name and tag |
| `dockerfile` | string | build | Path to Dockerfile (default `.`) |
| `context` | string | build | Build context directory (default `.`) |
| `build_args` | string[] | build | Build arguments (`--build-arg`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Examples

```toml
[project]
name = "my-docker-project"

[recipe.build]
type       = "docker-build"
image      = "myapp:latest"
dockerfile = "Dockerfile.prod"
context    = "."
build_args = ["VERSION=2.0", "ENV=prod"]

[recipe.push]
type  = "docker-push"
image = "myregistry/myapp:latest"
deps  = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
