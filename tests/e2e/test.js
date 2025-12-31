// E2E tests for session compatibility between Rust and Node.js
// Uses Node.js built-in test runner (Node 18+)

const { describe, it, before, after } = require('node:test');
const assert = require('node:assert');

const RUST_URL = process.env.RUST_URL || 'http://127.0.0.1:5800';
const NODE_URL = process.env.NODE_URL || 'http://127.0.0.1:3000';

// Helper to make HTTP requests with cookie handling
class TestClient {
  constructor() {
    this.cookies = new Map();
  }

  parseCookies(setCookieHeader) {
    if (!setCookieHeader) return;
    const cookies = Array.isArray(setCookieHeader) ? setCookieHeader : [setCookieHeader];
    for (const cookie of cookies) {
      const [nameValue] = cookie.split(';');
      const [name, value] = nameValue.split('=');
      this.cookies.set(name.trim(), value);
    }
  }

  getCookieHeader() {
    const cookies = [];
    for (const [name, value] of this.cookies) {
      cookies.push(`${name}=${value}`);
    }
    return cookies.join('; ');
  }

  async request(url) {
    const headers = {};
    if (this.cookies.size > 0) {
      headers['Cookie'] = this.getCookieHeader();
    }

    const response = await fetch(url, { headers });
    const setCookie = response.headers.get('set-cookie');
    if (setCookie) {
      this.parseCookies(setCookie);
    }

    return {
      status: response.status,
      headers: response.headers,
      body: await response.json()
    };
  }

  clearCookies() {
    this.cookies.clear();
  }
}

describe('Server Health Checks', () => {
  it('Rust server is healthy', async () => {
    const response = await fetch(`${RUST_URL}/health`);
    assert.strictEqual(response.status, 200);
  });

  it('Node.js server is healthy', async () => {
    const response = await fetch(`${NODE_URL}/health`);
    assert.strictEqual(response.status, 200);
    const body = await response.json();
    assert.strictEqual(body.server, 'nodejs');
  });
});

describe('Session Created in Node.js, Read in Rust', () => {
  const client = new TestClient();

  it('should create session in Node.js', async () => {
    const res = await client.request(`${NODE_URL}/set?key=user&value=alice`);
    assert.strictEqual(res.status, 200);
    assert.strictEqual(res.body.server, 'nodejs');
    assert.strictEqual(res.body.key, 'user');
    assert.strictEqual(res.body.value, 'alice');
    assert.ok(res.body.sessionId, 'Should have session ID');
  });

  it('should read session in Rust with same cookie', async () => {
    const res = await client.request(`${RUST_URL}/get?key=user`);
    assert.strictEqual(res.status, 200);
    assert.strictEqual(res.body.server, 'rust');
    assert.strictEqual(res.body.key, 'user');
    assert.strictEqual(res.body.value, 'alice');
    assert.strictEqual(res.body.exists, true);
  });

  it('should verify lastModifiedBy is nodejs', async () => {
    const res = await client.request(`${RUST_URL}/get?key=lastModifiedBy`);
    assert.strictEqual(res.body.value, 'nodejs');
  });
});

describe('Session Created in Rust, Read in Node.js', () => {
  const client = new TestClient();

  it('should create session in Rust', async () => {
    const res = await client.request(`${RUST_URL}/set?key=language&value=rust`);
    assert.strictEqual(res.status, 200);
    assert.strictEqual(res.body.server, 'rust');
    assert.strictEqual(res.body.key, 'language');
    assert.strictEqual(res.body.value, 'rust');
  });

  it('should read session in Node.js with same cookie', async () => {
    const res = await client.request(`${NODE_URL}/get?key=language`);
    assert.strictEqual(res.status, 200);
    assert.strictEqual(res.body.server, 'nodejs');
    assert.strictEqual(res.body.value, 'rust');
    assert.strictEqual(res.body.exists, true);
  });

  it('should verify lastModifiedBy is rust', async () => {
    const res = await client.request(`${NODE_URL}/get?key=lastModifiedBy`);
    assert.strictEqual(res.body.value, 'rust');
  });
});

describe('Cross-Platform Session Modifications', () => {
  const client = new TestClient();

  it('should create session with counter in Node.js', async () => {
    const res = await client.request(`${NODE_URL}/counter`);
    assert.strictEqual(res.body.counter, 1);
    assert.strictEqual(res.body.server, 'nodejs');
  });

  it('should increment counter in Rust', async () => {
    const res = await client.request(`${RUST_URL}/counter`);
    assert.strictEqual(res.body.counter, 2);
    assert.strictEqual(res.body.server, 'rust');
  });

  it('should increment counter in Node.js again', async () => {
    const res = await client.request(`${NODE_URL}/counter`);
    assert.strictEqual(res.body.counter, 3);
  });

  it('should increment counter in Rust again', async () => {
    const res = await client.request(`${RUST_URL}/counter`);
    assert.strictEqual(res.body.counter, 4);
  });

  it('should read final counter value from both servers', async () => {
    const nodeRes = await client.request(`${NODE_URL}/get?key=counter`);
    const rustRes = await client.request(`${RUST_URL}/get?key=counter`);
    
    assert.strictEqual(nodeRes.body.value, 4);
    assert.strictEqual(rustRes.body.value, 4);
  });
});

describe('Session Data Types', () => {
  const client = new TestClient();

  it('should handle string values', async () => {
    await client.request(`${NODE_URL}/set?key=stringVal&value=hello`);
    const res = await client.request(`${RUST_URL}/get?key=stringVal`);
    assert.strictEqual(res.body.value, 'hello');
  });

  it('should handle numeric string values', async () => {
    await client.request(`${RUST_URL}/set?key=numVal&value=12345`);
    const res = await client.request(`${NODE_URL}/get?key=numVal`);
    assert.strictEqual(res.body.value, '12345');
  });

  it('should handle special characters', async () => {
    const specialValue = encodeURIComponent('hello world & foo=bar');
    await client.request(`${NODE_URL}/set?key=special&value=${specialValue}`);
    const res = await client.request(`${RUST_URL}/get?key=special`);
    assert.strictEqual(res.body.value, 'hello world & foo=bar');
  });

  it('should handle unicode characters', async () => {
    const unicodeValue = encodeURIComponent('你好世界 🌍');
    await client.request(`${RUST_URL}/set?key=unicode&value=${unicodeValue}`);
    const res = await client.request(`${NODE_URL}/get?key=unicode`);
    assert.strictEqual(res.body.value, '你好世界 🌍');
  });
});

describe('Session Persistence', () => {
  const client = new TestClient();
  let sessionId;

  it('should create a session and remember its ID', async () => {
    const res = await client.request(`${NODE_URL}/set?key=persistent&value=data`);
    sessionId = res.body.sessionId;
    assert.ok(sessionId);
  });

  it('should maintain same session ID across requests to Node.js', async () => {
    const res = await client.request(`${NODE_URL}/session`);
    assert.strictEqual(res.body.sessionId, sessionId);
  });

  it('should maintain same session ID when switching to Rust', async () => {
    const res = await client.request(`${RUST_URL}/session`);
    assert.strictEqual(res.body.sessionId, sessionId);
  });
});

describe('New Session for New Client', () => {
  it('should create different sessions for different clients', async () => {
    const client1 = new TestClient();
    const client2 = new TestClient();

    const res1 = await client1.request(`${NODE_URL}/set?key=client&value=one`);
    const res2 = await client2.request(`${NODE_URL}/set?key=client&value=two`);

    assert.notStrictEqual(res1.body.sessionId, res2.body.sessionId);

    // Verify isolation - each client sees their own data
    const check1 = await client1.request(`${RUST_URL}/get?key=client`);
    const check2 = await client2.request(`${RUST_URL}/get?key=client`);

    assert.strictEqual(check1.body.value, 'one');
    assert.strictEqual(check2.body.value, 'two');
  });
});

console.log('Running E2E tests...');
console.log(`Rust server: ${RUST_URL}`);
console.log(`Node.js server: ${NODE_URL}`);
