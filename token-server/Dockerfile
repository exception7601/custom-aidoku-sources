FROM oven/bun:1.1.38

WORKDIR /app

# Instala wireguard-tools e dependencias
RUN apt-get update && \
    apt-get install -y --no-install-recommends wireguard-tools iproute2 openresolv iptables procps && \
    rm -rf /var/lib/apt/lists/*

# Copia package.json
COPY package.json ./

# Instala dependências
RUN bun install --production

# Copia código fonte
COPY src ./src

# Copia script de setup do wireguard
COPY setup-wireguard.sh /usr/local/bin/setup-wireguard.sh
RUN chmod +x /usr/local/bin/setup-wireguard.sh

# Expõe porta
EXPOSE 4000

# Variáveis de ambiente
ENV PORT=4000
ENV NODE_ENV=production

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD bun run -e 'fetch("http://localhost:4000/health").then(r => r.ok ? process.exit(0) : process.exit(1)).catch(() => process.exit(1))'

# Comando de inicialização com setup do wireguard
CMD ["/bin/bash", "-c", "/usr/local/bin/setup-wireguard.sh && bun run src/server.ts"]
