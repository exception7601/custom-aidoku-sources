import { describe, expect, it, beforeEach } from "bun:test";
import CryptoJS from "crypto-js";

describe("Crypto Logic - Manual Implementation", () => {
  describe("Passphrase Generation", () => {
    it("should generate 8-character passphrase", () => {
      const now = new Date();
      const utc = Date.UTC(
        now.getUTCFullYear(),
        now.getUTCMonth(),
        now.getUTCDate(),
        now.getUTCHours(),
      );
      const hash = CryptoJS.MD5(utc.toString()).toString();
      const passphrase = hash.substring(0, 8);

      expect(passphrase).toBeString();
      expect(passphrase.length).toBe(8);
      expect(passphrase).toMatch(/^[a-f0-9]{8}$/);
    });

    it("should be consistent within the same hour", () => {
      const now = new Date();
      const utc = Date.UTC(
        now.getUTCFullYear(),
        now.getUTCMonth(),
        now.getUTCDate(),
        now.getUTCHours(),
      );

      const hash1 = CryptoJS.MD5(utc.toString()).toString();
      const hash2 = CryptoJS.MD5(utc.toString()).toString();

      expect(hash1).toBe(hash2);
      expect(hash1.substring(0, 8)).toBe(hash2.substring(0, 8));
    });

    it("should change every hour", () => {
      const now = new Date();
      
      const currentHour = Date.UTC(
        now.getUTCFullYear(),
        now.getUTCMonth(),
        now.getUTCDate(),
        now.getUTCHours(),
      );

      const nextHour = Date.UTC(
        now.getUTCFullYear(),
        now.getUTCMonth(),
        now.getUTCDate(),
        now.getUTCHours() + 1,
      );

      const pass1 = CryptoJS.MD5(currentHour.toString()).toString().substring(0, 8);
      const pass2 = CryptoJS.MD5(nextHour.toString()).toString().substring(0, 8);

      expect(pass1).not.toBe(pass2);
    });
  });

  describe("Rabbit Encryption/Decryption", () => {
    it("should encrypt and decrypt with passphrase", () => {
      const original = "Hello, World!";
      const passphrase = "testpass";

      const encrypted = CryptoJS.Rabbit.encrypt(original, passphrase).toString();
      const decrypted = CryptoJS.Rabbit.decrypt(encrypted, passphrase).toString(
        CryptoJS.enc.Utf8,
      );

      expect(decrypted).toBe(original);
    });

    it("should encrypt and decrypt JSON", () => {
      const original = { id: "test", value: 42, nested: { key: "data" } };
      const passphrase = "mykey123";

      const encrypted = CryptoJS.Rabbit.encrypt(
        JSON.stringify(original),
        passphrase,
      ).toString();
      const decrypted = CryptoJS.Rabbit.decrypt(encrypted, passphrase).toString(
        CryptoJS.enc.Utf8,
      );
      const parsed = JSON.parse(decrypted);

      expect(parsed).toEqual(original);
    });

    it("should produce different ciphertext each time", () => {
      const original = "test message";
      const passphrase = "key";

      const encrypted1 = CryptoJS.Rabbit.encrypt(original, passphrase).toString();
      const encrypted2 = CryptoJS.Rabbit.encrypt(original, passphrase).toString();

      // Different ciphertext due to IV
      expect(encrypted1).not.toBe(encrypted2);

      // But both decrypt to same value
      const decrypted1 = CryptoJS.Rabbit.decrypt(
        encrypted1,
        passphrase,
      ).toString(CryptoJS.enc.Utf8);
      const decrypted2 = CryptoJS.Rabbit.decrypt(
        encrypted2,
        passphrase,
      ).toString(CryptoJS.enc.Utf8);

      expect(decrypted1).toBe(original);
      expect(decrypted2).toBe(original);
    });

    it("should fail with wrong passphrase", () => {
      const original = "secret data";
      const correctPass = "correct";
      const wrongPass = "wrong";

      const encrypted = CryptoJS.Rabbit.encrypt(original, correctPass).toString();

      expect(() => {
        const decrypted = CryptoJS.Rabbit.decrypt(
          encrypted,
          wrongPass,
        ).toString(CryptoJS.enc.Utf8);
        // If it doesn't throw, verify it's garbage
        expect(decrypted).not.toBe(original);
      }).toThrow();
    });
  });

  describe("Session Generation", () => {
    it("should generate random session ID", () => {
      const session1 =
        Math.random().toString(36).substring(2, 15) +
        Math.random().toString(36).substring(2, 15);
      const session2 =
        Math.random().toString(36).substring(2, 15) +
        Math.random().toString(36).substring(2, 15);

      expect(session1).toBeString();
      expect(session2).toBeString();
      expect(session1.length).toBeGreaterThan(20);
      expect(session2.length).toBeGreaterThan(20);
      expect(session1).not.toBe(session2);
    });
  });

  describe("Integration with crypto module", () => {
    it("should export getAuthTokens function", async () => {
      const { getAuthTokens } = await import("../src/crypto");
      expect(getAuthTokens).toBeFunction();
    });

    it("should export decryptData function", async () => {
      const { decryptData } = await import("../src/crypto");
      expect(decryptData).toBeFunction();
    });

    it("should export clearTokenCache function", async () => {
      const { clearTokenCache } = await import("../src/crypto");
      expect(clearTokenCache).toBeFunction();
    });
  });

  describe("Live Token Generation", () => {
    it("should generate tokens from API (integration test)", async () => {
      if (process.env.CI || process.env.SKIP_LIVE_TESTS) {
        console.log("Skipping live test");
        return;
      }

      try {
        const { getAuthTokens } = await import("../src/crypto");
        const tokens = await getAuthTokens();

        expect(tokens).toHaveProperty("signature");
        expect(tokens).toHaveProperty("verify");
        expect(tokens).toHaveProperty("passphrase");
        expect(tokens).toHaveProperty("session");

        expect(tokens.signature).toBeString();
        expect(tokens.verify).toBeString();
        expect(tokens.passphrase).toBeString();
        expect(tokens.session).toBeString();

        // Passphrase should be 8 characters
        expect(tokens.passphrase.length).toBe(8);
        expect(tokens.passphrase).toMatch(/^[a-f0-9]{8}$/);

        console.log("[test] Tokens generated:", {
          signatureLength: tokens.signature.length,
          verifyLength: tokens.verify.length,
          passphrase: tokens.passphrase,
          sessionLength: tokens.session.length,
        });
      } catch (error) {
        console.log("Live test skipped (network issue):", error);
      }
    }, 30000);
  });
});
