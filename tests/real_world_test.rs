//! Real-world code tests.
//!
//! Tests instrumentation on complex, production-like JavaScript and TypeScript
//! patterns to catch edge cases and verify the output is valid.

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn default_opts() -> InstrumentOptions {
    InstrumentOptions::default()
}

fn assert_valid_instrumentation(source: &str, filename: &str) {
    let result = instrument(source, filename, &default_opts()).unwrap();
    // Verify code is not empty
    assert!(
        !result.code.is_empty(),
        "Instrumented code should not be empty"
    );
    // Verify coverage map has a path
    assert_eq!(result.coverage_map.path, filename);
    // Verify hit counts match map sizes
    assert_eq!(
        result.coverage_map.s.len(),
        result.coverage_map.statement_map.len()
    );
    assert_eq!(
        result.coverage_map.f.len(),
        result.coverage_map.fn_map.len()
    );
    assert_eq!(
        result.coverage_map.b.len(),
        result.coverage_map.branch_map.len()
    );
    // Verify branch hit count arrays match location counts
    for (id, entry) in &result.coverage_map.branch_map {
        let hit_counts = &result.coverage_map.b[id];
        assert_eq!(
            hit_counts.len(),
            entry.locations.len(),
            "Branch {id} hit count array length should match locations"
        );
    }
    // Verify coverage map serializes to valid JSON
    let json = serde_json::to_string(&result.coverage_map).unwrap();
    let _parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
}

#[test]
fn real_world_react_component() {
    let source = r#"
import React, { useState, useEffect } from 'react';

const UserProfile = ({ userId, onUpdate }) => {
  const [user, setUser] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    const fetchUser = async () => {
      try {
        setLoading(true);
        const response = await fetch(`/api/users/${userId}`);
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }
        const data = await response.json();
        setUser(data);
      } catch (err) {
        setError(err.message || 'Unknown error');
      } finally {
        setLoading(false);
      }
    };
    fetchUser();
  }, [userId]);

  const handleSubmit = (e) => {
    e.preventDefault();
    if (user && onUpdate) {
      onUpdate(user);
    }
  };

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error}</div>;
  if (!user) return null;

  return (
    <form onSubmit={handleSubmit}>
      <input value={user.name || ''} />
      <button type="submit">Save</button>
    </form>
  );
};

export default UserProfile;
"#;
    assert_valid_instrumentation(source, "UserProfile.jsx");
}

#[test]
fn real_world_express_middleware() {
    let source = r"
const express = require('express');
const jwt = require('jsonwebtoken');

const authMiddleware = (req, res, next) => {
  const token = req.headers.authorization?.split(' ')[1];

  if (!token) {
    return res.status(401).json({ error: 'No token provided' });
  }

  try {
    const decoded = jwt.verify(token, process.env.JWT_SECRET);
    req.user = decoded;
    next();
  } catch (err) {
    if (err.name === 'TokenExpiredError') {
      return res.status(401).json({ error: 'Token expired' });
    }
    return res.status(403).json({ error: 'Invalid token' });
  }
};

const rateLimit = (maxRequests, windowMs) => {
  const requests = new Map();

  return (req, res, next) => {
    const ip = req.ip ?? req.connection.remoteAddress;
    const now = Date.now();
    const windowStart = now - windowMs;

    const userRequests = requests.get(ip) || [];
    const recentRequests = userRequests.filter(t => t > windowStart);

    if (recentRequests.length >= maxRequests) {
      return res.status(429).json({ error: 'Too many requests' });
    }

    recentRequests.push(now);
    requests.set(ip, recentRequests);
    next();
  };
};

module.exports = { authMiddleware, rateLimit };
";
    assert_valid_instrumentation(source, "middleware.js");
}

#[test]
fn real_world_typescript_service() {
    let source = r"
interface User {
  id: string;
  name: string;
  email: string;
  role: 'admin' | 'user' | 'guest';
}

interface QueryOptions {
  limit?: number;
  offset?: number;
  sortBy?: keyof User;
}

class UserService {
  private users: Map<string, User> = new Map();

  async findById(id: string): Promise<User | null> {
    return this.users.get(id) ?? null;
  }

  async findAll(options: QueryOptions = {}): Promise<User[]> {
    const { limit = 10, offset = 0, sortBy = 'name' } = options;
    let result = Array.from(this.users.values());

    if (sortBy) {
      result.sort((a, b) => {
        const aVal = a[sortBy];
        const bVal = b[sortBy];
        return aVal < bVal ? -1 : aVal > bVal ? 1 : 0;
      });
    }

    return result.slice(offset, offset + limit);
  }

  async create(data: Omit<User, 'id'>): Promise<User> {
    const id = crypto.randomUUID();
    const user: User = { id, ...data };
    this.users.set(id, user);
    return user;
  }

  async update(id: string, data: Partial<User>): Promise<User | null> {
    const existing = this.users.get(id);
    if (!existing) return null;

    const updated = { ...existing, ...data };
    this.users.set(id, updated);
    return updated;
  }

  async delete(id: string): Promise<boolean> {
    return this.users.delete(id);
  }

  async findByRole(role: User['role']): Promise<User[]> {
    return Array.from(this.users.values()).filter(u => u.role === role);
  }
}

export { UserService };
export type { User, QueryOptions };
";
    assert_valid_instrumentation(source, "UserService.ts");
}

#[test]
fn real_world_complex_control_flow() {
    let source = r"
function processPayment(order, paymentMethod) {
  const amount = order.items.reduce((sum, item) => sum + item.price * item.quantity, 0);
  const tax = amount * (order.taxRate ?? 0.1);
  const total = amount + tax;

  if (total <= 0) {
    return { success: false, error: 'Invalid amount' };
  }

  switch (paymentMethod.type) {
    case 'credit_card': {
      if (!paymentMethod.cardNumber || !paymentMethod.cvv) {
        return { success: false, error: 'Missing card details' };
      }
      const isValid = paymentMethod.cardNumber.length === 16 && paymentMethod.cvv.length === 3;
      if (!isValid) {
        return { success: false, error: 'Invalid card' };
      }
      break;
    }
    case 'paypal': {
      if (!paymentMethod.email) {
        return { success: false, error: 'Missing PayPal email' };
      }
      break;
    }
    case 'bank_transfer': {
      if (!paymentMethod.iban) {
        return { success: false, error: 'Missing IBAN' };
      }
      break;
    }
    default:
      return { success: false, error: 'Unsupported payment method' };
  }

  for (const item of order.items) {
    if (item.quantity <= 0) {
      return { success: false, error: `Invalid quantity for ${item.name}` };
    }
  }

  const receipt = {
    orderId: order.id,
    amount,
    tax,
    total,
    paymentMethod: paymentMethod.type,
    timestamp: new Date().toISOString(),
    items: order.items.map(item => ({
      name: item.name,
      quantity: item.quantity,
      subtotal: item.price * item.quantity,
    })),
  };

  return { success: true, receipt };
}
";
    assert_valid_instrumentation(source, "payment.js");
}

#[test]
fn real_world_async_generators_and_iterators() {
    let source = r"
async function* paginate(url, pageSize = 10) {
  let page = 0;
  let hasMore = true;

  while (hasMore) {
    const response = await fetch(`${url}?page=${page}&size=${pageSize}`);
    const data = await response.json();

    if (data.items.length === 0) {
      hasMore = false;
    } else {
      yield* data.items;
      page++;
      hasMore = data.items.length === pageSize;
    }
  }
}

const processItems = async () => {
  const results = [];
  for await (const item of paginate('/api/items')) {
    if (item.status === 'active') {
      results.push(item);
    }
    if (results.length >= 100) break;
  }
  return results;
};
";
    assert_valid_instrumentation(source, "paginate.js");
}

#[test]
fn real_world_class_with_private_fields() {
    let source = r#"
class EventEmitter {
  #listeners = new Map();
  #maxListeners = 10;

  on(event, handler) {
    if (!this.#listeners.has(event)) {
      this.#listeners.set(event, []);
    }

    const handlers = this.#listeners.get(event);
    if (handlers.length >= this.#maxListeners) {
      console.warn(`Max listeners (${this.#maxListeners}) exceeded for "${event}"`);
    }

    handlers.push(handler);
    return this;
  }

  off(event, handler) {
    const handlers = this.#listeners.get(event);
    if (!handlers) return this;

    const index = handlers.indexOf(handler);
    if (index !== -1) {
      handlers.splice(index, 1);
    }
    return this;
  }

  emit(event, ...args) {
    const handlers = this.#listeners.get(event);
    if (!handlers || handlers.length === 0) return false;

    for (const handler of handlers) {
      try {
        handler(...args);
      } catch (err) {
        console.error(`Error in handler for "${event}":`, err);
      }
    }
    return true;
  }
}
"#;
    assert_valid_instrumentation(source, "EventEmitter.js");
}

#[test]
fn real_world_destructuring_and_spread() {
    let source = r"
function mergeConfig(defaults, ...overrides) {
  const result = { ...defaults };

  for (const override_ of overrides) {
    const { enabled = true, ...rest } = override_ ?? {};
    if (enabled) {
      Object.assign(result, rest);
    }
  }

  const { debug, verbose, ...production } = result;
  return process.env.NODE_ENV === 'production' ? production : result;
}

const [first, second, ...remaining] = [1, 2, 3, 4, 5];
const combined = first + second + remaining.reduce((a, b) => a + b, 0);
";
    assert_valid_instrumentation(source, "config.js");
}

#[test]
fn real_world_typescript_generics() {
    let source = r"
type Result<T, E = Error> = { ok: true; data: T } | { ok: false; error: E };

function tryCatch<T>(fn: () => T): Result<T> {
  try {
    return { ok: true, data: fn() };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error : new Error(String(error)) };
  }
}

async function tryAsync<T>(fn: () => Promise<T>): Promise<Result<T>> {
  try {
    const data = await fn();
    return { ok: true, data };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error : new Error(String(error)) };
  }
}

class Repository<T extends { id: string }> {
  private items: Map<string, T> = new Map();

  add(item: T): void {
    this.items.set(item.id, item);
  }

  get(id: string): T | undefined {
    return this.items.get(id);
  }

  findWhere(predicate: (item: T) => boolean): T[] {
    return Array.from(this.items.values()).filter(predicate);
  }
}
";
    assert_valid_instrumentation(source, "result.ts");
}

#[test]
fn real_world_coverage_map_is_valid_istanbul_format() {
    let source = r"
function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

const memoize = (fn) => {
  const cache = new Map();
  return (...args) => {
    const key = JSON.stringify(args);
    if (cache.has(key)) return cache.get(key);
    const result = fn(...args);
    cache.set(key, result);
    return result;
  };
};

const fastFib = memoize(fibonacci);
";
    let result = instrument(source, "fibonacci.js", &default_opts()).unwrap();

    // Verify Istanbul format: root should be keyed by path
    let mut root = std::collections::BTreeMap::new();
    root.insert(result.coverage_map.path.clone(), &result.coverage_map);
    let root_json = serde_json::to_string_pretty(&root).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&root_json).unwrap();

    // Should have the file path as key
    assert!(parsed["fibonacci.js"].is_object());
    assert_eq!(parsed["fibonacci.js"]["path"], "fibonacci.js");

    // All required Istanbul fields present
    let cov = &parsed["fibonacci.js"];
    for field in &["path", "statementMap", "fnMap", "branchMap", "s", "f", "b"] {
        assert!(
            cov[field].is_string() || cov[field].is_object(),
            "Missing Istanbul field: {field}"
        );
    }
}
