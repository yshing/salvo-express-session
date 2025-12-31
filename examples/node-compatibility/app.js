// Node.js example showing compatibility with salvo-express-session
// This demonstrates that sessions created by Rust can be read by Node.js and vice versa

const express = require('express');
const session = require('express-session');
const { createClient } = require('redis');
const RedisStore = require('connect-redis').default;

async function main() {
  // Create Redis client
  const redisClient = createClient({
    url: process.env.REDIS_URL || 'redis://127.0.0.1/'
  });
  
  await redisClient.connect();
  console.log('Connected to Redis');

  // Create Redis store
  const redisStore = new RedisStore({
    client: redisClient,
    prefix: 'sess:', // Same as Rust default
  });

  const app = express();

  // Configure session - MUST match Rust configuration for compatibility
  app.use(session({
    store: redisStore,
    secret: process.env.SESSION_SECRET || 'keyboard cat', // Same secret as Rust
    name: 'connect.sid', // Same cookie name as Rust
    resave: false,
    saveUninitialized: false,
    cookie: {
      maxAge: 86400 * 1000, // 1 day in milliseconds
      httpOnly: true,
      secure: false,
    }
  }));

  // View counter
  app.get('/', (req, res) => {
    req.session.views = (req.session.views || 0) + 1;
    res.send(`
      <h1>Hello from Node.js!</h1>
      <p>Views: ${req.session.views}</p>
      <p>Session ID: ${req.sessionID}</p>
      <p>This session is compatible with Rust salvo-express-session!</p>
      <p><a href="/data">View raw session data</a></p>
    `);
  });

  // Show raw session data
  app.get('/data', (req, res) => {
    res.json({
      sessionId: req.sessionID,
      session: req.session
    });
  });

  // Set custom data
  app.get('/set', (req, res) => {
    const key = req.query.key || 'test';
    const value = req.query.value || 'hello';
    req.session[key] = value;
    res.send(`Set ${key}=${value}`);
  });

  // Start server on different port to run alongside Rust
  const PORT = process.env.PORT || 3000;
  app.listen(PORT, () => {
    console.log(`Node.js server running at http://127.0.0.1:${PORT}`);
    console.log('');
    console.log('Endpoints:');
    console.log('  GET /           - View counter');
    console.log('  GET /data       - Show raw session data');
    console.log('  GET /set?key=x&value=y - Set session data');
    console.log('');
    console.log('To test compatibility:');
    console.log('1. Start this Node.js app on port 3000');
    console.log('2. Start the Rust app on port 5800');
    console.log('3. Visit http://127.0.0.1:3000 to create a session');
    console.log('4. Copy the connect.sid cookie value');
    console.log('5. Visit http://127.0.0.1:5800 with the same cookie');
    console.log('6. Both apps will share the same session!');
  });
}

main().catch(console.error);
