FROM node:20-alpine
WORKDIR /app
COPY services/web/package*.json ./
RUN npm ci --production
COPY services/web/ .
EXPOSE 8080
CMD ["node", "app.js"]
