// Node.js Express server for E2E testing
// This server must be compatible with the Rust salvo-express-session

const express = require('express');
const session = require('express-session');
const { createClient } = require('redis');
const RedisStore = require('connect-redis').default;

const PORT = process.env.PORT || 3000;
const REDIS_URL = process.env.REDIS_URL || 'redis://127.0.0.1:6379';
const SESSION_SECRET = process.env.SESSION_SECRET || 'e2e-test-secret-key';

async function main() {
  // Create Redis client
  const redisClient = createClient({ url: REDIS_URL });
  
  redisClient.on('error', (err) => console.error('Redis Client Error', err));
  
  await redisClient.connect();
  console.log('Connected to Redis at', REDIS_URL);

  // Create Redis store with same settings as Rust app
  const redisStore = new RedisStore({
    client: redisClient,
    prefix: 'sess:', // Must match Rust prefix
  });

  const app = express();

  // Configure session - MUST match Rust configuration exactly
  app.use(session({
    store: redisStore,
    secret: SESSION_SECRET,
    name: 'connect.sid', // Must match Rust cookie name
    resave: false,
    saveUninitialized: false,
    cookie: {
      maxAge: 86400 * 1000, // 1 day in milliseconds
      httpOnly: true,
      secure: false,
      sameSite: 'lax',
    }
  }));

  // Health check endpoint
  app.get('/health', (req, res) => {
    res.json({ status: 'ok', server: 'nodejs' });
  });

  // Get session info
  app.get('/session', (req, res) => {
    res.json({
      server: 'nodejs',
      sessionId: req.sessionID,
      isNew: req.session.isNew !== false,
      data: { ...req.session }
    });
  });

  // Set a value in session
  app.get('/set', (req, res) => {
    const key = req.query.key || 'testKey';
    const value = req.query.value || 'testValue';
    req.session[key] = value;
    req.session.lastModifiedBy = 'nodejs';
    req.session.lastModifiedAt = new Date().toISOString();
    res.json({
      server: 'nodejs',
      action: 'set',
      key,
      value,
      sessionId: req.sessionID
    });
  });

  // Get a value from session
  app.get('/get', (req, res) => {
    const key = req.query.key || 'testKey';
    const value = req.session[key];
    res.json({
      server: 'nodejs',
      action: 'get',
      key,
      value: value !== undefined ? value : null,
      exists: value !== undefined,
      sessionId: req.sessionID,
      lastModifiedBy: req.session.lastModifiedBy || null
    });
  });

  // Increment counter
  app.get('/counter', (req, res) => {
    req.session.counter = (req.session.counter || 0) + 1;
    req.session.lastModifiedBy = 'nodejs';
    res.json({
      server: 'nodejs',
      counter: req.session.counter,
      sessionId: req.sessionID
    });
  });

  // Clear session data
  app.get('/clear', (req, res) => {
    const sessionId = req.sessionID;
    req.session.destroy((err) => {
      if (err) {
        res.status(500).json({ error: err.message });
      } else {
        res.json({
          server: 'nodejs',
          action: 'clear',
          previousSessionId: sessionId
        });
      }
    });
  });

  // Get cookie info
  app.get('/cookie-info', (req, res) => {
    res.json({
      server: 'nodejs',
      sessionId: req.sessionID,
      cookie: {
        originalMaxAge: req.session.cookie.originalMaxAge,
        maxAge: req.session.cookie.maxAge,
        expires: req.session.cookie.expires,
        httpOnly: req.session.cookie.httpOnly,
        secure: req.session.cookie.secure,
        path: req.session.cookie.path,
        sameSite: req.session.cookie.sameSite,
        domain: req.session.cookie.domain
      }
    });
  });

  // Set custom cookie max age (dynamic expiration)
  app.get('/set-cookie-maxage', (req, res) => {
    const maxAgeSecs = parseInt(req.query.seconds || '3600', 10);
    req.session.cookie.maxAge = maxAgeSecs * 1000; // express uses milliseconds
    req.session.customMaxAgeSet = true;
    req.session.lastModifiedBy = 'nodejs';
    res.json({
      server: 'nodejs',
      action: 'set-cookie-maxage',
      maxAgeSecs,
      newExpires: req.session.cookie.expires,
      sessionId: req.sessionID
    });
  });

  app.listen(PORT, () => {
    console.log(`Node.js E2E test server running on http://127.0.0.1:${PORT}`);
    console.log(`Using session secret: ${SESSION_SECRET.substring(0, 4)}...`);
  });
}

main().catch(console.error);
