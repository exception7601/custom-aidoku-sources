FROM oven/bun:1.1.38-alpine

WORKDIR /app

# Copia package.json
COPY package.json ./

# Instala dependências
RUN bun install --production

# Copia código fonte
COPY src ./src

# Expõe porta
EXPOSE 3000

# Variáveis de ambiente
ENV PORT=3000
ENV NODE_ENV=production

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD bun run -e 'fetch("http://localhost:3000/health").then(r => r.ok ? process.exit(0) : process.exit(1)).catch(() => process.exit(1))'

# Comando de inicialização
CMD ["bun", "run", "src/server.ts"]
