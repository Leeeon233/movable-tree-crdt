/*
 * License CC0 - http://creativecommons.org/publicdomain/zero/1.0/
 * To the extent possible under law, the author(s) have dedicated all
 * copyright and related and neighboring rights to this software to
 * the public domain worldwide. This software is distributed without
 * any warranty.
 */

class Tree {
  constructor(db) {
    const newNodeWithID = (id) => ({
      id,
      parent: null,
      children: [],
      edges: new Map(),
      cycle: null,
    });
    this.db = db;
    this.root = newNodeWithID("(ROOT)");
    this.nodes = new Map();
    this.nodes.set(this.root.id, this.root);

    // Keep the in-memory tree up to date and cycle-free as the database is mutated
    db.afterApply(({ op }) => {
      // Each mutation takes place on the child. The key is the parent
      // identifier and the value is the counter for that graph edge.
      let child = this.nodes.get(op.id);
      if (!child) {
        // Make sure the child exists
        child = newNodeWithID(op.id);
        this.nodes.set(op.id, child);
      }
      if (!this.nodes.has(op.key)) {
        // Make sure the parent exists
        this.nodes.set(op.key, newNodeWithID(op.key));
      }
      if (op.value === undefined) {
        // Undo can revert a value back to undefined
        child.edges.delete(op.key);
      } else {
        // Otherwise, add an edge from the child to the parent
        child.edges.set(op.key, op.value);
      }
      this.recomputeParentsAndChildren();
    });
  }

  recomputeParentsAndChildren() {
    // Start off with all children arrays empty and each parent pointer
    // for a given node set to the most recent edge for that node.
    for (const node of this.nodes.values()) {
      // Set the parent identifier to the link with the largest counter
      node.parent = this.nodes.get(edgeWithLargestCounter(node)) || null;
      node.children = [];
    }

    // At this point all nodes that can reach the root form a tree (by
    // construction, since each node other than the root has a single
    // parent). The parent pointers for the remaining nodes may form one
    // or more cycles. Gather all remaining nodes detached from the root.
    const nonRootedNodes = new Set();
    for (let node of this.nodes.values()) {
      if (!isNodeUnderOtherNode(node, this.root)) {
        while (node && !nonRootedNodes.has(node)) {
          nonRootedNodes.add(node);
          node = node.parent;
        }
      }
    }

    // Deterministically reattach these nodes to the tree under the root
    // node. The order of reattachment is arbitrary but needs to be based
    // only on information in the database so that all peers reattach
    // non-rooted nodes in the same order and end up with the same tree.
    if (nonRootedNodes.size > 0) {
      // All "ready" edges already have the parent connected to the root,
      // and all "deferred" edges have a parent not yet connected to the
      // root. Prioritize newer edges over older edges using the counter.
      const deferredEdges = new Map();
      const readyEdges = new PriorityQueue((a, b) => {
        const counterDelta = b.counter - a.counter;
        if (counterDelta !== 0) return counterDelta;
        if (a.parent.id < b.parent.id) return -1;
        if (a.parent.id > b.parent.id) return 1;
        if (a.child.id < b.child.id) return -1;
        if (a.child.id > b.child.id) return 1;
        return 0;
      });
      for (const child of nonRootedNodes) {
        for (const [parentID, counter] of child.edges) {
          const parent = this.nodes.get(parentID);
          if (!nonRootedNodes.has(parent)) {
            readyEdges.push({ child, parent, counter });
          } else {
            let edges = deferredEdges.get(parent);
            if (!edges) {
              edges = [];
              deferredEdges.set(parent, edges);
            }
            edges.push({ child, parent, counter });
          }
        }
      }
      for (let top; (top = readyEdges.pop()); ) {
        // Skip nodes that have already been reattached
        const child = top.child;
        if (!nonRootedNodes.has(child)) continue;

        // Reattach this node
        child.parent = top.parent;
        nonRootedNodes.delete(child);

        // Activate all deferred edges for this node
        const edges = deferredEdges.get(child);
        if (edges) for (const edge of edges) readyEdges.push(edge);
      }
    }

    // Add items as children of their parents so that the rest of the app
    // can easily traverse down the tree for drawing and hit-testing
    for (const node of this.nodes.values()) {
      if (node.parent) {
        node.parent.children.push(node);
      }
    }

    // Sort each node's children by their identifiers so that all peers
    // display the same tree. In this demo, the ordering of siblings
    // under the same parent is considered unimportant. If this is
    // important for your app, you will need to use another CRDT in
    // combination with this CRDT to handle the ordering of siblings.
    for (const node of this.nodes.values()) {
      node.children.sort((a, b) => {
        if (a.id < b.id) return -1;
        if (a.id > b.id) return 1;
        return 0;
      });
    }
  }

  addChildToParent(childID, parentID) {
    const ensureNodeIsRooted = (child) => {
      while (child) {
        const parent = child.parent;
        if (!parent) break;
        const edge = edgeWithLargestCounter(child);
        if (edge !== parent.id) edits.push([child, parent]);
        child = parent;
      }
    };

    // Ensure that both the old and new parents remain where they are
    // in the tree after the edit we are about to make. Then move the
    // child from its old parent to its new parent.
    const edits = [];
    const child = this.nodes.get(childID);
    const parent = this.nodes.get(parentID);
    ensureNodeIsRooted(child.parent);
    ensureNodeIsRooted(parent);
    edits.push([child, parent]);

    // Apply all database edits accumulated above. If your database
    // supports syncing a set of changes in a single batch, then these
    // edits should all be part of the same batch for efficiency. The
    // order that these edits are made in shouldn't matter.
    for (const [child, parent] of edits) {
      let maxCounter = -1;
      for (const counter of child.edges.values()) {
        maxCounter = Math.max(maxCounter, counter);
      }
      this.db.set(child.id, parent.id, maxCounter + 1);
    }
  }
}

// Note: This priority queue implementation is inefficient. It should
// probably be implemented using a heap instead. This only matters when
// there area large numbers of edges on nodes involved in cycles.
class PriorityQueue {
  constructor(compare) {
    this.compare = compare;
    this.items = [];
  }
  push(item) {
    this.items.push(item);
    this.items.sort(this.compare);
  }
  pop() {
    return this.items.shift();
  }
}

// The edge with the largest counter is considered to be the most recent
// one. If two edges are set simultaneously, the identifier breaks the tie.
function edgeWithLargestCounter(node) {
  let edgeID = null;
  let largestCounter = -1;
  for (const [id, counter] of node.edges) {
    if (
      counter > largestCounter ||
      (counter === largestCounter && id > edgeID)
    ) {
      edgeID = id;
      largestCounter = counter;
    }
  }
  return edgeID;
}

// Returns true if and only if "node" is in the subtree under "other".
// This function is safe to call in the presence of parent cycles.
function isNodeUnderOtherNode(node, other) {
  if (node === other) return true;
  let tortoise = node;
  let hare = node.parent;
  while (hare && hare !== other) {
    if (tortoise === hare) return false; // Cycle detected
    hare = hare.parent;
    if (!hare || hare === other) break;
    tortoise = tortoise.parent;
    hare = hare.parent;
  }
  return hare === other;
}

// The code above is everything relevant to the mutable tree hierarchy
// algorithm. The remaining code provides the UI for the demo.

const canvas = document.querySelector("canvas");
const isMobile = document.body.clientWidth < 800;
let previousFrame = Date.now();
let buttons = [];

class Peer {
  constructor(id) {
    this.id = id;
    this.x = 0;
    this.y = 0;
    this.w = 0;
    this.h = 0;
    this.peerX = 0;
    this.peerY = 0;
    this.db = new DB(id);
    this.tree = new Tree(this.db);
    this.undoRedo = new UndoRedo(this.db);
    this.animatedTreeNodes = new Map();
    this.draggingID = null;
    this.draggingMouse = null;
    this.dropID = null;
  }

  updateAnimations(seconds) {
    const t = 1 - Math.pow(0.01, seconds);
    const visit = (node, x, y) => {
      let width = 0;
      for (const child of node.children) {
        width += visit(child, x + width, y + 50) + 50;
      }
      width = Math.max(width - 50, 0);
      x += width / 2;
      let info = this.animatedTreeNodes.get(node.id);
      if (!info) {
        const oldParent =
          node.parent && this.animatedTreeNodes.get(node.parent.id);
        info = oldParent
          ? { x: oldParent.x, y: oldParent.y, width: 0 }
          : { x, y, width: 0 };
      }
      info.x += (x - info.x) * t;
      info.y += (y - info.y) * t;
      info.width += (width - info.width) * t;
      info.targetX = x;
      info.targetY = y;
      this.animatedTreeNodes.set(node.id, info);
      return width;
    };
    visit(this.tree.root, 0, 0);
  }

  draw(context) {
    const style = getComputedStyle(document.body);
    const textColor = style.color;
    const backgroundColor = style.backgroundColor;
    const { x, y, w, h, peerX, peerY } = this;

    // Clip all drawing to this peer's rectangle
    context.save();
    context.beginPath();
    context.translate(x, y);
    context.rect(0, 0, w, h);
    context.clip();

    // Draw the peer identifier
    context.save();
    if (!enableNetwork) context.globalAlpha = 0.5;
    context.fillStyle = textColor;
    context.textAlign = "center";
    context.textBaseline = y ? "top" : "bottom";
    context.fillText(this.id, peerX - x, peerY - y + (y ? 20 : -20));
    context.beginPath();
    context.arc(peerX - x, peerY - y, 10, 0, 2 * Math.PI, false);
    context.fill();
    context.restore();

    // Draw the graph edges
    let offsetY = 30;
    context.textBaseline = "middle";
    for (const node of this.tree.nodes.values()) {
      const sign = x ? -1 : 1;
      let offsetX = x ? w - 30 : 30;
      context.fillStyle = arbitraryColorFromID(node.id);
      context.beginPath();
      context.arc(offsetX, offsetY, 8, 0, 2 * Math.PI, false);
      context.fill();
      context.strokeStyle = textColor;
      context.fillStyle = textColor;
      context.beginPath();
      context.moveTo(offsetX + 15 * sign, offsetY);
      context.lineTo(offsetX + 20 * sign, offsetY);
      context.stroke();
      context.beginPath();
      context.moveTo(offsetX + 25 * sign, offsetY);
      context.lineTo(offsetX + 20 * sign, offsetY - 3);
      context.lineTo(offsetX + 20 * sign, offsetY + 3);
      context.fill();
      offsetX += 30 * sign;
      const edges = node.edges;
      if (!edges.size) {
        context.textAlign = x ? "right" : "left";
        context.fillText("ⁿ/ₐ", offsetX, offsetY);
      } else {
        const sorted = [...edges.keys()].sort((a, b) => {
          const counterDelta = edges.get(b) - edges.get(a);
          if (counterDelta !== 0) return counterDelta;
          if (a < b) return -1;
          if (a > b) return 1;
          return 0;
        });
        context.textAlign = "center";
        for (let i = 0; i < sorted.length; i++) {
          const edge = sorted[i];
          const radius = node.parent && node.parent.id === edge ? 5 : 1.5;
          const text = x
            ? (i + 1 < sorted.length ? " , " : "") + edges.get(edge) + " "
            : " " + edges.get(edge) + (i + 1 < sorted.length ? ", " : "");
          const width = context.measureText(text).width;
          context.fillStyle = arbitraryColorFromID(edge);
          context.beginPath();
          context.arc(
            offsetX + radius * sign,
            offsetY,
            radius,
            0,
            2 * Math.PI,
            false
          );
          context.fill();
          offsetX += radius * 2 * sign;
          context.fillStyle = textColor;
          context.fillText(text, offsetX + (width / 2) * sign, offsetY);
          offsetX += width * sign;
        }
      }
      offsetY += 20;
    }

    // Draw the tree
    const visit = (node) => {
      const { x: nodeX, y: nodeY } = this.animatedTreeNodes.get(node.id);

      // Draw the node's children
      for (const child of node.children) {
        const { x: childX, y: childY } = this.animatedTreeNodes.get(child.id);
        context.lineWidth = 1;
        context.beginPath();
        context.moveTo(treeX + nodeX, treeY + nodeY);
        context.lineTo(treeX + childX, treeY + childY);
        context.stroke();
        context.fillStyle = textColor;
        context.save();
        context.translate(treeX + nodeX, treeY + nodeY);
        context.rotate(Math.atan2(nodeX - childX, childY - nodeY));
        context.beginPath();
        context.moveTo(0, 15);
        context.lineTo(-5, 25);
        context.lineTo(5, 25);
        context.fill();
        context.restore();
        visit(child);
      }

      // If we're currently dragging this entry, draw an arrow
      if (node.id === this.draggingID) {
        let { x: targetX, y: targetY } = this.draggingMouse;
        targetX -= x;
        targetY -= y;
        this.dropID = null;
        for (const [id, parent] of this.tree.nodes) {
          if (node.parent === parent || isNodeUnderOtherNode(parent, node)) {
            // To match the behavior of a real app, prevent dropping node
            // X as a child of node Y if either a) Y is already the parent
            // of X or b) Y is in the subtree of X (since that would be
            // a cycle). Real apps will enforce this so the demo does too.
            continue;
          }
          const { x: otherX, y: otherY } = this.animatedTreeNodes.get(id);
          if (
            id !== node.id &&
            distanceBetween(targetX, targetY, treeX + otherX, treeY + otherY) <
              15
          ) {
            const distance = distanceBetween(nodeX, nodeY, otherX, otherY);
            const t = (distance - 15) / distance;
            targetX = treeX + nodeX + (otherX - nodeX) * t;
            targetY = treeY + nodeY + (otherY - nodeY) * t;
            this.dropID = id;
            break;
          }
        }
        context.lineWidth = 1;
        context.beginPath();
        context.moveTo(treeX + nodeX, treeY + nodeY);
        context.lineTo(targetX, targetY);
        context.stroke();
        context.fillStyle = textColor;
        context.save();
        context.translate(targetX, targetY);
        context.rotate(
          Math.atan2(treeX + nodeX - targetX, targetY - treeY - nodeY)
        );
        context.beginPath();
        context.moveTo(0, 0);
        context.lineTo(-5, -10);
        context.lineTo(5, -10);
        context.fill();
        context.restore();
      }

      // Draw the node itself
      context.fillStyle = arbitraryColorFromID(node.id);
      context.beginPath();
      context.arc(treeX + nodeX, treeY + nodeY, 15, 0, 2 * Math.PI, false);
      context.fill();
      if (node.id === this.draggingID) {
        context.strokeStyle = textColor;
        context.lineWidth = 4;
        context.stroke();
      }

      if (node !== this.tree.root) {
        buttons.push({
          cursor: "move",
          x: x + treeX + nodeX,
          y: y + treeY + nodeY,
          radius: 15,
          dragAction: {
            start: () => {
              this.draggingID = node.id;
              this.dropID = null;
            },
            move: (e) => {
              this.draggingMouse = mouse(e);
              document.body.style.cursor = "move";
            },
            stop: () => {
              if (this.draggingID !== null && this.dropID !== null) {
                // Group all parent pointer mutations in this operation into a single
                // undo/redo batch so they are all undone and redone in a single step
                this.undoRedo.batch(() => {
                  this.tree.addChildToParent(this.draggingID, this.dropID);
                });
              }
              this.draggingID = null;
            },
          },
        });
      }
    };
    const treeX =
      Math.round((w * (x ? 2 : 3)) / 5) -
      this.animatedTreeNodes.get(this.tree.root.id).width / 2;
    const treeY = 50;
    context.strokeStyle = textColor;
    context.lineWidth = 1;
    visit(this.tree.root);

    // Draw the undo history
    const { undoHistory, redoHistory } = this.undoRedo;
    const undoRedoLength = undoHistory.length + redoHistory.length;
    if (undoRedoLength > 0) {
      let offset = 80;
      if (x) {
        offset = w - offset;
        for (const changes of undoHistory) offset -= changes.length * 8 + 12;
        for (const changes of redoHistory) offset -= changes.length * 8 + 12;
      }
      context.fillStyle = textColor;
      context.textAlign = "center";
      context.textBaseline = "middle";
      context.fillText("Undo", offset - 30, h - 40);
      buttons.push({
        cursor: "pointer",
        x: x + offset - 30,
        y: y + h - 40,
        radius: 20,
        action: () => this.undoRedo.undo(),
      });
      for (let i = 0; i < undoHistory.length; i++) {
        const changes = undoHistory[i];
        for (let j = 0; j < changes.length; j++) {
          const change = changes[j];
          context.fillStyle = backgroundColor;
          context.beginPath();
          context.arc(offset + 10, h - 40, 10, 0, 2 * Math.PI, false);
          context.fill();
          context.fillStyle = arbitraryColorFromID(change.id);
          context.beginPath();
          context.arc(offset + 10, h - 40, 8, 0, Math.PI, false);
          context.fill();
          context.fillStyle = arbitraryColorFromID(change.key);
          context.beginPath();
          context.arc(offset + 10, h - 40, 8, 0, Math.PI, true);
          context.fill();
          offset += 8;
        }
        offset += 12;
      }
      context.fillStyle = textColor;
      context.beginPath();
      context.moveTo(offset, h - 30);
      context.lineTo(offset - 5, h - 25);
      context.lineTo(offset + 5, h - 25);
      context.fill();
      for (let i = redoHistory.length - 1; i >= 0; i--) {
        const changes = redoHistory[i];
        for (let j = changes.length - 1; j >= 0; j--) {
          const change = changes[j];
          context.fillStyle = backgroundColor;
          context.beginPath();
          context.arc(offset + 10, h - 40, 10, 0, 2 * Math.PI, false);
          context.fill();
          context.fillStyle = arbitraryColorFromID(change.id);
          context.beginPath();
          context.arc(offset + 10, h - 40, 8, 0, Math.PI, false);
          context.fill();
          context.fillStyle = arbitraryColorFromID(change.key);
          context.beginPath();
          context.arc(offset + 10, h - 40, 8, 0, Math.PI, true);
          context.fill();
          offset += 8;
        }
        offset += 12;
      }
      context.fillStyle = textColor;
      context.fillText("Redo", offset + 30, h - 40);
      buttons.push({
        cursor: "pointer",
        x: x + offset + 30,
        y: y + h - 40,
        radius: 20,
        action: () => this.undoRedo.redo(),
      });
    }

    // Draw a border around this peer
    context.globalAlpha = 0.5;
    context.strokeStyle = textColor;
    context.lineWidth = 6;
    context.strokeRect(isMobile ? 0 : x ? -1 : 1, y ? -1 : 1, w, h);

    context.restore();
  }
}

function arbitraryColorFromID(id) {
  let hash = 0;
  for (const c of id) hash = Math.imul(hash ^ c.charCodeAt(0), 0x1000193);
  return "#" + (0x1000000 | hash).toString(16).slice(-6);
}

const peers = isMobile
  ? [new Peer("Peer 1"), new Peer("Peer 2")]
  : [
      new Peer("Peer 1"),
      new Peer("Peer 2"),
      new Peer("Peer 3"),
      new Peer("Peer 4"),
    ];

// Seed some initial data
setTimeout(() => {
  peers[0].db.set("abk0980oz3", peers[0].tree.root.id, 0);
  peers[0].undoRedo.undoHistory = [];
}, 100);
setTimeout(() => {
  peers[0].db.set("tkr24y7uli", peers[0].tree.root.id, 0);
  peers[0].undoRedo.undoHistory = [];
}, 200);
setTimeout(() => {
  peers[0].db.set("dopsc4mbaup", "abk0980oz3", 0);
  peers[0].undoRedo.undoHistory = [];
}, 300);
setTimeout(() => {
  peers[0].db.set("p97h88b4lgr", "abk0980oz3", 0);
  peers[0].undoRedo.undoHistory = [];
}, 400);
setTimeout(() => {
  peers[0].db.set("akumsplmgeq", "tkr24y7uli", 0);
  peers[0].undoRedo.undoHistory = [];
}, 500);
setTimeout(() => {
  peers[0].db.set("yas5wscaid9", "tkr24y7uli", 0);
  peers[0].undoRedo.undoHistory = [];
}, 600);

// Simulate network traffic
let enableNetwork = true;
const updateNetworkSim = (seconds) => {
  const oldPackets = packets;
  packets = [];
  for (const packet of oldPackets) {
    if (enableNetwork) packet.life += seconds;
    if (packet.life < packet.lifetime) packets.push(packet);
    else
      for (const peer of peers)
        if (peer.id === packet.to) peer.db.apply(packet.op, "remote");
  }
};
let packets = [];
for (const a of peers) {
  for (const b of peers) {
    if (a !== b) {
      a.db.afterApply(({ op, origin }) => {
        if (origin !== "remote")
          packets.push({ op, from: a.id, to: b.id, life: 0.05, lifetime: 0.5 });
      });
    }
  }
}

function draw() {
  const width = isMobile
    ? Math.min(600, document.body.clientWidth - 2 * 30)
    : Math.min(1200, document.body.clientWidth - 2 * 50);
  const height = isMobile ? Math.round(width * 2) : Math.round(width * 0.75);
  const ratio = window.devicePixelRatio || 1;
  canvas.width = Math.round(width * ratio);
  canvas.height = Math.round(height * ratio);
  canvas.style.width = width + "px";
  canvas.style.height = height + "px";
  const context = canvas.getContext("2d");
  context.scale(ratio, ratio);
  const style = getComputedStyle(document.body);
  const textColor = style.color;
  context.font = style.font;
  buttons = [];

  // Draw each peer quadrant
  if (isMobile) {
    for (let i = 0; i < peers.length; i++) {
      const peer = peers[i];
      peer.x = 0;
      peer.y = Math.round((height * i) / 2);
      peer.w = width;
      peer.h = Math.round((height * (i + 1)) / 2) - peer.y;
      peer.peerX = peer.x + (peer.x ? 50 : peer.w - 50);
      peer.peerY = peer.y + (peer.y ? 50 : peer.h - 50);
      peer.draw(context);
    }
  } else {
    for (let i = 0; i < peers.length; i++) {
      const peer = peers[i];
      peer.x = Math.round((width * (i & 1)) / 2);
      peer.y = Math.round((height * (i >> 1)) / 2);
      peer.w = Math.round((width * ((i & 1) + 1)) / 2) - peer.x;
      peer.h = Math.round((height * ((i >> 1) + 1)) / 2) - peer.y;
      peer.peerX = peer.x + (peer.x ? 50 : peer.w - 50);
      peer.peerY = peer.y + (peer.y ? 50 : peer.h - 50);
      peer.draw(context);
    }
  }

  // Draw the paths between peers
  if (enableNetwork) {
    context.save();
    context.globalAlpha = 0.5;
    context.strokeStyle = textColor;
    context.setLineDash([5, 5]);
    context.beginPath();
    context.lineWidth = 2;
    for (let i = 0; i < peers.length; i++) {
      for (let j = i + 1; j < peers.length; j++) {
        context.moveTo(peers[i].peerX, peers[i].peerY);
        context.lineTo(peers[j].peerX, peers[j].peerY);
      }
    }
    context.stroke();
    context.restore();
  }

  // Draw the network packets in transit
  context.save();
  if (!enableNetwork) context.globalAlpha = 0.5;
  for (const packet of packets) {
    let from, to;
    for (const peer of peers) {
      if (peer.id === packet.from) from = peer;
      if (peer.id === packet.to) to = peer;
    }
    const t = packet.life / packet.lifetime;
    context.fillStyle = textColor;
    context.beginPath();
    context.arc(
      from.peerX + (to.peerX - from.peerX) * t,
      from.peerY + (to.peerY - from.peerY) * t,
      5,
      0,
      2 * Math.PI,
      false
    );
    context.fill();
  }
  context.restore();

  // Draw the enable network toggle
  const toggleX = isMobile ? width - 50 : width >> 1;
  const toggleY = height >> 1;
  const text = "Network paused";
  const textWidth = context.measureText(text).width;
  const textX = isMobile ? -textWidth / 2 - 40 : 0;
  const textY = isMobile ? 0 : 40;
  context.translate(toggleX, toggleY);
  context.save();
  context.fillStyle = "white";
  context.shadowColor = "rgba(0, 0, 0, 0.5)";
  context.shadowOffsetY = 3;
  context.shadowBlur = 6;
  context.beginPath();
  context.arc(0, 0, 20, 0, 2 * Math.PI, false);
  context.fill();
  if (!enableNetwork) {
    context.strokeStyle = "white";
    context.lineCap = "round";
    context.lineWidth = 24;
    context.beginPath();
    context.moveTo(textX - textWidth / 2, textY);
    context.lineTo(textX + textWidth / 2, textY);
    context.stroke();
  }
  context.restore();
  if (enableNetwork) {
    context.fillRect(-8, -10, 6, 20);
    context.fillRect(2, -10, 6, 20);
  } else {
    context.beginPath();
    context.moveTo(10, 0);
    context.lineTo(-6, -10);
    context.lineTo(-6, 10);
    context.fill();
    context.fillStyle = "#222";
    context.textAlign = "center";
    context.textBaseline = "middle";
    context.fillText(text, textX, textY);
  }
  buttons.push({
    cursor: "pointer",
    x: toggleX,
    y: toggleY,
    radius: 20,
    action: () => (enableNetwork = !enableNetwork),
  });
}

function tick() {
  const currentFrame = Date.now();
  const seconds =
    currentFrame - previousFrame < 500
      ? (currentFrame - previousFrame) / 1000
      : 0;
  previousFrame = currentFrame;
  requestAnimationFrame(tick);
  updateNetworkSim(seconds);
  for (const peer of peers) peer.updateAnimations(seconds);
  draw();
}

function mouse(e) {
  let x = e.pageX;
  let y = e.pageY;
  for (let el = canvas; el !== null; el = el.offsetParent) {
    x -= el.offsetLeft;
    y -= el.offsetTop;
  }
  return { x, y };
}

function distanceBetween(ax, ay, bx, by) {
  const x = bx - ax;
  const y = by - ay;
  return Math.sqrt(x * x + y * y);
}

onmousemove = (e) => {
  const { x, y } = mouse(e);
  for (const button of buttons) {
    if (distanceBetween(x, y, button.x, button.y) < button.radius) {
      document.body.style.cursor = button.cursor;
      return;
    }
  }
  document.body.style.cursor = "auto";
};

onmousedown = (e) => {
  const { x, y } = mouse(e);
  for (const button of buttons) {
    if (distanceBetween(x, y, button.x, button.y) < button.radius) {
      e.preventDefault();
      if (button.dragAction) {
        const mousemove = onmousemove;
        button.dragAction.start();
        onmousemove = (e) => button.dragAction.move(e);
        onmousemove(e);
        onmouseup = (e) => {
          button.dragAction.stop();
          onmousemove = mousemove;
          onmousemove(e);
          onmouseup = null;
        };
      } else {
        button.action(e);
      }
      return;
    }
  }
};

addEventListener(
  "touchstart",
  (e) => {
    if (e.touches.length === 1) {
      const { x, y } = mouse(e.touches[0]);
      for (const button of buttons) {
        if (
          distanceBetween(x, y, button.x, button.y) <
          button.radius * (isMobile ? 1.5 : 1)
        ) {
          e.preventDefault();
          if (button.dragAction) {
            button.dragAction.start();
            const onmove = (e) => {
              e.preventDefault();
              if (e.touches.length === 1) {
                button.dragAction.move(e.touches[0]);
              }
            };
            addEventListener("touchmove", onmove, { passive: false });
            onmove(e);
            ontouchend = (e) => {
              ontouchend = null;
              removeEventListener("touchmove", onmove, { passive: false });
              button.dragAction.stop();
            };
          } else {
            ontouchmove = (e) => {
              if (e.touches.length !== 1) {
                // This was a pinch, not a tap
                ontouchmove = ontouchend = null;
              } else {
                const { x: newX, y: newY } = mouse(e.touches[0]);
                if (distanceBetween(newX, newY, x, y) > 1) {
                  // This was a drag, not a tap
                  ontouchmove = ontouchend = null;
                }
              }
            };
            ontouchend = (e) => {
              ontouchmove = ontouchend = null;
              button.action(e);
            };
          }
          return;
        }
      }
    }
  },
  { passive: false }
);

// Disable double-tap to zoom in Mobile Safari
canvas.ondblclick = (e) => {
  e.preventDefault();
};

onresize = draw;
tick();

try {
  // Newer browsers
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", draw);
} catch (e) {
  // Older browsers
  matchMedia("(prefers-color-scheme: dark)").addListener(draw);
}
