interface AccessLog {
  id: string;
  timestamp: string;
  ip: string;
  method: string;
  path: string;
  status: number;
  responseTime: number;
  userAgent?: string;
  error?: string;
}

class AccessLogger {
  private logs: AccessLog[] = [];
  private readonly MAX_LOGS = 1000;

  log(entry: Omit<AccessLog, "id" | "timestamp">): void {
    const log: AccessLog = {
      id: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
      ...entry,
    };

    this.logs.push(log);
    if (this.logs.length > this.MAX_LOGS) {
      this.logs = this.logs.slice(-this.MAX_LOGS);
    }

    console.log(
      `[access] ${entry.method} ${entry.path} - ${entry.status} - ${entry.responseTime}ms - ${entry.ip}`,
    );
  }

  getLogs(limit?: number): AccessLog[] {
    const logs = [...this.logs].reverse();
    return limit ? logs.slice(0, limit) : logs;
  }

  getStats(): {
    total: number;
    byStatus: Record<number, number>;
    byPath: Record<string, number>;
    avgResponseTime: number;
    errors: number;
  } {
    const stats = {
      total: this.logs.length,
      byStatus: {} as Record<number, number>,
      byPath: {} as Record<string, number>,
      avgResponseTime: 0,
      errors: 0,
    };

    let totalResponseTime = 0;

    for (const log of this.logs) {
      stats.byStatus[log.status] = (stats.byStatus[log.status] || 0) + 1;
      stats.byPath[log.path] = (stats.byPath[log.path] || 0) + 1;
      totalResponseTime += log.responseTime;
      if (log.status >= 400 || log.error) {
        stats.errors++;
      }
    }

    stats.avgResponseTime =
      stats.total > 0 ? Math.round(totalResponseTime / stats.total) : 0;

    return stats;
  }

  clear(): void {
    this.logs = [];
    console.log("[logger] Logs cleared");
  }
}

export const accessLogger = new AccessLogger();
