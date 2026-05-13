FROM node:20-alpine
WORKDIR /app
COPY services/api/package*.json ./
RUN npm ci --production
COPY services/api/ .
EXPOSE 3000
CMD ["node", "app.js"]
