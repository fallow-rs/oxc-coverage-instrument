// Large module (~500 lines) stress test for benchmarking
// Simulates a state management library with complex control flow

class Store {
  #state = {};
  #listeners = new Map();
  #middleware = [];
  #history = [];
  #maxHistory = 100;
  #devtools = null;
  #batching = false;
  #pendingUpdates = [];

  constructor(initialState = {}, options = {}) {
    this.#state = { ...initialState };
    this.#maxHistory = options.maxHistory ?? 100;
    if (options.devtools) {
      this.#devtools = options.devtools;
      this.#devtools.init(this.#state);
    }
    if (options.middleware) {
      for (const mw of options.middleware) {
        if (typeof mw === 'function') {
          this.#middleware.push(mw);
        } else if (mw && typeof mw.handler === 'function') {
          this.#middleware.push(mw.handler);
        }
      }
    }
  }

  getState() {
    return { ...this.#state };
  }

  select(selector) {
    if (typeof selector === 'string') {
      return this.#state[selector];
    } else if (typeof selector === 'function') {
      return selector(this.#state);
    }
    return undefined;
  }

  subscribe(key, listener) {
    if (typeof key === 'function') {
      listener = key;
      key = '*';
    }
    if (!this.#listeners.has(key)) {
      this.#listeners.set(key, new Set());
    }
    this.#listeners.get(key).add(listener);

    return () => {
      const set = this.#listeners.get(key);
      if (set) {
        set.delete(listener);
        if (set.size === 0) {
          this.#listeners.delete(key);
        }
      }
    };
  }

  dispatch(action) {
    if (!action || typeof action !== 'object') {
      throw new TypeError('Action must be an object');
    }
    if (!action.type || typeof action.type !== 'string') {
      throw new TypeError('Action must have a string type');
    }

    if (this.#batching) {
      this.#pendingUpdates.push(action);
      return;
    }

    let processedAction = action;
    for (const mw of this.#middleware) {
      const result = mw(processedAction, this.#state);
      if (result === false) return;
      if (result && typeof result === 'object') {
        processedAction = result;
      }
    }

    const prevState = { ...this.#state };
    this.#applyAction(processedAction);
    this.#recordHistory(prevState, processedAction);
    this.#notifyListeners(prevState);

    if (this.#devtools) {
      this.#devtools.send(processedAction, this.#state);
    }
  }

  #applyAction(action) {
    switch (action.type) {
      case 'SET':
        if (action.key && action.value !== undefined) {
          this.#state[action.key] = action.value;
        }
        break;
      case 'MERGE':
        if (action.payload && typeof action.payload === 'object') {
          this.#state = { ...this.#state, ...action.payload };
        }
        break;
      case 'DELETE':
        if (action.key) {
          delete this.#state[action.key];
        }
        break;
      case 'RESET':
        this.#state = action.payload ?? {};
        break;
      case 'TRANSFORM':
        if (typeof action.transform === 'function') {
          const newState = action.transform(this.#state);
          if (newState && typeof newState === 'object') {
            this.#state = newState;
          }
        }
        break;
      default:
        if (action.reducer && typeof action.reducer === 'function') {
          const result = action.reducer(this.#state, action);
          if (result !== undefined) {
            this.#state = result;
          }
        }
        break;
    }
  }

  #recordHistory(prevState, action) {
    this.#history.push({
      prevState,
      action,
      timestamp: Date.now(),
    });
    while (this.#history.length > this.#maxHistory) {
      this.#history.shift();
    }
  }

  #notifyListeners(prevState) {
    const globalListeners = this.#listeners.get('*');
    if (globalListeners) {
      for (const listener of globalListeners) {
        try {
          listener(this.#state, prevState);
        } catch (err) {
          console.error('Listener error:', err);
        }
      }
    }

    for (const [key, listeners] of this.#listeners) {
      if (key === '*') continue;
      if (this.#state[key] !== prevState[key]) {
        for (const listener of listeners) {
          try {
            listener(this.#state[key], prevState[key]);
          } catch (err) {
            console.error(`Listener error for key "${key}":`, err);
          }
        }
      }
    }
  }

  batch(fn) {
    this.#batching = true;
    try {
      fn(this);
    } finally {
      this.#batching = false;
      const updates = [...this.#pendingUpdates];
      this.#pendingUpdates = [];
      for (const action of updates) {
        this.dispatch(action);
      }
    }
  }

  undo() {
    if (this.#history.length === 0) return false;
    const entry = this.#history.pop();
    if (!entry) return false;
    const prevState = { ...this.#state };
    this.#state = entry.prevState;
    this.#notifyListeners(prevState);
    return true;
  }

  getHistory() {
    return this.#history.map(entry => ({
      action: entry.action,
      timestamp: entry.timestamp,
    }));
  }
}

function createDerived(store, selector, equalityFn) {
  let cachedValue = selector(store.getState());
  const listeners = new Set();

  const eq = equalityFn ?? ((a, b) => a === b);

  store.subscribe((state, prevState) => {
    const newValue = selector(state);
    if (!eq(newValue, cachedValue)) {
      const oldValue = cachedValue;
      cachedValue = newValue;
      for (const listener of listeners) {
        listener(newValue, oldValue);
      }
    }
  });

  return {
    get value() { return cachedValue; },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

function combineStores(...stores) {
  const combined = new Store();

  for (let i = 0; i < stores.length; i++) {
    const store = stores[i];
    const key = store.name ?? `store_${i}`;

    combined.dispatch({
      type: 'SET',
      key,
      value: store.getState(),
    });

    store.subscribe((state) => {
      combined.dispatch({
        type: 'SET',
        key,
        value: state,
      });
    });
  }

  return combined;
}

function createAsyncAction(type, asyncFn) {
  return async (store, ...args) => {
    store.dispatch({ type: `${type}_PENDING` });
    try {
      const result = await asyncFn(...args);
      store.dispatch({ type: `${type}_FULFILLED`, payload: result });
      return result;
    } catch (error) {
      store.dispatch({
        type: `${type}_REJECTED`,
        error: error.message ?? String(error),
      });
      throw error;
    }
  };
}

const loggerMiddleware = (action, state) => {
  console.log(`[${new Date().toISOString()}] ${action.type}`, action);
  return action;
};

const freezeMiddleware = (action, state) => {
  if (action.type === 'FREEZE') return false;
  return action;
};

const validationMiddleware = (validators) => {
  return (action, state) => {
    const validator = validators[action.type];
    if (!validator) return action;

    const errors = validator(action, state);
    if (errors && errors.length > 0) {
      console.warn(`Validation failed for ${action.type}:`, errors);
      return false;
    }
    return action;
  };
};

function persistMiddleware(storage, key) {
  return (action, state) => {
    try {
      const serialized = JSON.stringify(state);
      storage.setItem(key, serialized);
    } catch (err) {
      console.error('Failed to persist state:', err);
    }
    return action;
  };
}

function loadPersistedState(storage, key) {
  try {
    const serialized = storage.getItem(key);
    if (serialized === null || serialized === undefined) return null;
    return JSON.parse(serialized);
  } catch (err) {
    console.error('Failed to load persisted state:', err);
    return null;
  }
}

class ComputedValue {
  #fn;
  #deps;
  #cache = undefined;
  #dirty = true;
  #store;

  constructor(store, deps, fn) {
    this.#store = store;
    this.#deps = deps;
    this.#fn = fn;

    for (const dep of deps) {
      store.subscribe(dep, () => {
        this.#dirty = true;
      });
    }
  }

  get value() {
    if (this.#dirty) {
      const state = this.#store.getState();
      const depValues = this.#deps.map(d => state[d]);
      this.#cache = this.#fn(...depValues);
      this.#dirty = false;
    }
    return this.#cache;
  }
}

function shallowEqual(a, b) {
  if (a === b) return true;
  if (!a || !b) return false;
  if (typeof a !== 'object' || typeof b !== 'object') return false;

  const keysA = Object.keys(a);
  const keysB = Object.keys(b);

  if (keysA.length !== keysB.length) return false;

  for (const key of keysA) {
    if (a[key] !== b[key]) return false;
  }
  return true;
}

function deepEqual(a, b) {
  if (a === b) return true;
  if (a === null || b === null) return false;
  if (typeof a !== typeof b) return false;

  if (Array.isArray(a)) {
    if (!Array.isArray(b)) return false;
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!deepEqual(a[i], b[i])) return false;
    }
    return true;
  }

  if (typeof a === 'object') {
    const keysA = Object.keys(a);
    const keysB = Object.keys(b);
    if (keysA.length !== keysB.length) return false;
    for (const key of keysA) {
      if (!deepEqual(a[key], b[key])) return false;
    }
    return true;
  }

  return false;
}

module.exports = {
  Store,
  createDerived,
  combineStores,
  createAsyncAction,
  loggerMiddleware,
  freezeMiddleware,
  validationMiddleware,
  persistMiddleware,
  loadPersistedState,
  ComputedValue,
  shallowEqual,
  deepEqual,
};
