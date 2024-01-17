// This is a simple last-writer-wins peer-to-peer database for use with
// demos. It's in this separate file because all demos share this code.
class DB {
  constructor(peer) {
    this._peer = peer;
    this._rows = new Map();
    this._observers = [];
  }

  afterApply(observer) {
    this._observers.push(observer);
  }

  get ids() {
    return [...this._rows.keys()];
  }

  get(id, key) {
    const row = this._rows.get(id);
    const field = row && row.get(key);
    return field && field.value;
  }

  set(id, key, value) {
    const op = { id, key, value, peer: this._peer, timestamp: Date.now() };
    const row = this._rows.get(id);
    const field = row && row.get(key);
    if (field) {
      // Make sure "set" always overwrites locally.
      op.timestamp = Math.max(op.timestamp, field.timestamp + 1);
    }
    this.apply(op, "local");
  }

  apply(op, origin) {
    let row = this._rows.get(op.id);
    if (!row) {
      row = new Map();
      this._rows.set(op.id, row);
    }
    const field = row.get(op.key);
    if (
      field &&
      (field.timestamp > op.timestamp ||
        (field.timestamp === op.timestamp && field.peer > op.peer))
    ) {
      // Don't overwrite newer values with older values. The last writer always wins.
    } else {
      row.set(op.key, {
        peer: op.peer,
        timestamp: op.timestamp,
        value: op.value,
      });
    }
    for (const observer of this._observers) {
      observer({ op, origin, oldValue: field && field.value });
    }
  }
}

class UndoRedo {
  constructor(db, { onlyKeys } = {}) {
    this.db = db;
    this.undoHistory = [];
    this.redoHistory = [];
    this._onlyKeys = onlyKeys && new Set(onlyKeys);
    this._isBusy = false;
    this._pending = [];
    this._depth = 0;

    db.afterApply(({ op, origin, oldValue }) => {
      if (
        origin === "local" &&
        !this._isBusy &&
        (!this._onlyKeys || this._onlyKeys.has(op.key))
      ) {
        this._pending.push({ id: op.id, key: op.key, value: oldValue });
        this._commit();
      }
    });
  }

  batch(callback) {
    this._depth++;
    callback();
    this._depth--;
    this._commit();
  }

  undo() {
    const top = this.undoHistory.pop();
    if (top) this.redoHistory.push(this._apply(top));
  }

  redo() {
    const top = this.redoHistory.pop();
    if (top) this.undoHistory.push(this._apply(top));
  }

  _commit() {
    if (this._depth === 0) {
      this.undoHistory.push(this._pending);
      this.redoHistory = [];
      this._pending = [];
    }
  }

  _apply(changes) {
    const reverse = [];
    this._isBusy = true;
    for (const { id, key, value } of changes) {
      reverse.push({ id, key, value: this.db.get(id, key) });
      this.db.set(id, key, value);
    }
    this._isBusy = false;
    return reverse.reverse();
  }
}
