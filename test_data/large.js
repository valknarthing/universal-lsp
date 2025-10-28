// Large JavaScript file for benchmarking
// This file contains multiple classes and functions

class DataProcessor {
    constructor(config) {
        this.config = config || {};
        this.cache = new Map();
        this.stats = {
            processed: 0,
            errors: 0,
            cached: 0
        };
    }

    process(data) {
        if (!Array.isArray(data)) {
            throw new TypeError('Data must be an array');
        }

        return data.map(item => {
            try {
                return this.processItem(item);
            } catch (error) {
                this.stats.errors++;
                console.error('Error processing item:', error);
                return null;
            }
        }).filter(item => item !== null);
    }

    processItem(item) {
        const cacheKey = this.getCacheKey(item);

        if (this.cache.has(cacheKey)) {
            this.stats.cached++;
            return this.cache.get(cacheKey);
        }

        const result = this.transform(item);
        this.cache.set(cacheKey, result);
        this.stats.processed++;

        return result;
    }

    transform(item) {
        if (typeof item === 'string') {
            return item.trim().toLowerCase();
        }

        if (typeof item === 'number') {
            return Math.round(item * 100) / 100;
        }

        if (typeof item === 'object' && item !== null) {
            return Object.keys(item).reduce((acc, key) => {
                acc[key] = this.transform(item[key]);
                return acc;
            }, {});
        }

        return item;
    }

    getCacheKey(item) {
        return JSON.stringify(item);
    }

    getStats() {
        return { ...this.stats };
    }

    clearCache() {
        this.cache.clear();
        this.stats.cached = 0;
    }

    reset() {
        this.cache.clear();
        this.stats = {
            processed: 0,
            errors: 0,
            cached: 0
        };
    }
}

class EventEmitter {
    constructor() {
        this.events = new Map();
    }

    on(event, handler) {
        if (!this.events.has(event)) {
            this.events.set(event, []);
        }
        this.events.get(event).push(handler);
        return () => this.off(event, handler);
    }

    off(event, handler) {
        if (!this.events.has(event)) {
            return false;
        }

        const handlers = this.events.get(event);
        const index = handlers.indexOf(handler);

        if (index !== -1) {
            handlers.splice(index, 1);
            return true;
        }

        return false;
    }

    emit(event, ...args) {
        if (!this.events.has(event)) {
            return 0;
        }

        const handlers = this.events.get(event);
        handlers.forEach(handler => {
            try {
                handler(...args);
            } catch (error) {
                console.error('Error in event handler:', error);
            }
        });

        return handlers.length;
    }

    once(event, handler) {
        const wrappedHandler = (...args) => {
            handler(...args);
            this.off(event, wrappedHandler);
        };
        return this.on(event, wrappedHandler);
    }

    removeAllListeners(event) {
        if (event) {
            this.events.delete(event);
        } else {
            this.events.clear();
        }
    }

    listenerCount(event) {
        if (!this.events.has(event)) {
            return 0;
        }
        return this.events.get(event).length;
    }
}

class AsyncQueue {
    constructor(concurrency = 1) {
        this.concurrency = concurrency;
        this.running = 0;
        this.queue = [];
    }

    async add(fn) {
        return new Promise((resolve, reject) => {
            this.queue.push({ fn, resolve, reject });
            this.process();
        });
    }

    async process() {
        if (this.running >= this.concurrency || this.queue.length === 0) {
            return;
        }

        this.running++;
        const { fn, resolve, reject } = this.queue.shift();

        try {
            const result = await fn();
            resolve(result);
        } catch (error) {
            reject(error);
        } finally {
            this.running--;
            this.process();
        }
    }

    get size() {
        return this.queue.length;
    }

    get pending() {
        return this.running;
    }

    clear() {
        this.queue = [];
    }
}

// Utility functions
function debounce(fn, delay) {
    let timeoutId;
    return function(...args) {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => fn.apply(this, args), delay);
    };
}

function throttle(fn, limit) {
    let inThrottle;
    return function(...args) {
        if (!inThrottle) {
            fn.apply(this, args);
            inThrottle = true;
            setTimeout(() => inThrottle = false, limit);
        }
    };
}

function deepClone(obj) {
    if (obj === null || typeof obj !== 'object') {
        return obj;
    }

    if (obj instanceof Date) {
        return new Date(obj.getTime());
    }

    if (obj instanceof Array) {
        return obj.map(item => deepClone(item));
    }

    if (obj instanceof Object) {
        const clonedObj = {};
        Object.keys(obj).forEach(key => {
            clonedObj[key] = deepClone(obj[key]);
        });
        return clonedObj;
    }

    throw new Error('Unable to clone object');
}

function memoize(fn) {
    const cache = new Map();
    return function(...args) {
        const key = JSON.stringify(args);
        if (cache.has(key)) {
            return cache.get(key);
        }
        const result = fn.apply(this, args);
        cache.set(key, result);
        return result;
    };
}

// Export all classes and utilities
module.exports = {
    DataProcessor,
    EventEmitter,
    AsyncQueue,
    debounce,
    throttle,
    deepClone,
    memoize
};
