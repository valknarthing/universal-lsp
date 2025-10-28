// Medium-sized JavaScript file for benchmarking
class Calculator {
    constructor() {
        this.history = [];
    }

    add(a, b) {
        const result = a + b;
        this.history.push({ operation: 'add', a, b, result });
        return result;
    }

    subtract(a, b) {
        const result = a - b;
        this.history.push({ operation: 'subtract', a, b, result });
        return result;
    }

    multiply(a, b) {
        const result = a * b;
        this.history.push({ operation: 'multiply', a, b, result });
        return result;
    }

    divide(a, b) {
        if (b === 0) {
            throw new Error('Division by zero');
        }
        const result = a / b;
        this.history.push({ operation: 'divide', a, b, result });
        return result;
    }

    getHistory() {
        return this.history;
    }

    clearHistory() {
        this.history = [];
    }
}

// Helper functions
function formatNumber(num, decimals = 2) {
    return num.toFixed(decimals);
}

function isValidNumber(value) {
    return typeof value === 'number' && !isNaN(value) && isFinite(value);
}

// Export
module.exports = { Calculator, formatNumber, isValidNumber };
