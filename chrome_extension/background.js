// Leo Link - Background Worker
// Connects to Leo's local WebSocket server and executes commands

let socket = null;
let reconnectDelay = 1000;
const MAX_RECONNECT_DELAY = 30000;
const SERVER_URL = "ws://127.0.0.1:2345";

function safeSend(msg) {
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(msg));
  }
}

function connect() {
  console.log("Leo Link: Connecting to " + SERVER_URL);
  try {
    socket = new WebSocket(SERVER_URL);
  } catch (e) {
    console.error("Leo Link: Connection initialization failed", e);
    setTimeout(connect, reconnectDelay);
    return;
  }

  socket.onopen = () => {
    console.log("Leo Link: Connected! 🦁");
    reconnectDelay = 1000;
    chrome.action.setBadgeText({ text: "ON" });
    chrome.action.setBadgeBackgroundColor({ color: "#4CAF50" });
    safeSend({ type: "hello", message: "Extension connected" });
  };

  socket.onmessage = async (event) => {
    console.log("Leo Link: Command received:", event.data);
    try {
      const command = JSON.parse(event.data);
      handleCommand(command);
    } catch (e) {
      console.error("Leo Link: Failed to parse command", e);
    }
  };

  socket.onclose = () => {
    console.log(`Leo Link: Disconnected. Retrying in ${reconnectDelay / 1000}s...`);
    chrome.action.setBadgeText({ text: "OFF" });
    chrome.action.setBadgeBackgroundColor({ color: "#F44336" });
    socket = null;
    setTimeout(connect, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 1.5, MAX_RECONNECT_DELAY);
  };

  socket.onerror = (error) => {
    console.error("Leo Link: WebSocket Error", error);
    // onclose will handle reconnect
  };
}

async function handleCommand(cmd) {
  const action = cmd.action;
  safeSend({ type: "ack", action: action, status: "starting" });

  try {
    if (action === "open") {
      let url = cmd.url;
      if (!url.startsWith("http://") && !url.startsWith("https://")) {
        url = "https://" + url;
      }
      await chrome.tabs.create({ url: url });
      safeSend({ type: "result", action: action, status: "success", message: "Opened " + url });
    }
    else if (action === "screenshot" || action === "moment") {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      if (!tab) throw new Error("No active tab");

      const dataUrl = await chrome.tabs.captureVisibleTab(null, { format: "png" });
      let data = { screenshot: dataUrl };

      if (action === "moment") {
        const results = await chrome.scripting.executeScript({
          target: { tabId: tab.id },
          func: () => ({
            text: document.body.innerText,
            title: document.title,
            url: window.location.href,
            html: document.documentElement.outerHTML
          })
        });
        data.page = results[0].result;
      }
      safeSend({ type: "result", action: action, status: "success", data: data });
    }
    else if (action === "click" || action === "type" || action === "read" || action === "scroll" || action === "get_elements" || action === "wait") {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      if (!tab) throw new Error("No active tab found");

      // Inject Readability if we are reading
      if (action === "read") {
        await chrome.scripting.executeScript({
          target: { tabId: tab.id },
          files: ["lib/Readability.js"]
        });
      }

      const result = await chrome.scripting.executeScript({
        target: { tabId: tab.id },
        func: executePageAction,
        args: [cmd]
      });

      const output = result[0]?.result;
      safeSend({ type: "result", action: action, status: "success", data: output });
    }
  } catch (err) {
    safeSend({ type: "error", action: action, message: err.message });
  }
}

// This function runs INSIDE the web page
function executePageAction(cmd) {
  const highlightElement = (el) => {
    const originalOutline = el.style.outline;
    const originalTransition = el.style.transition;
    el.style.transition = 'outline 0.3s ease';
    el.style.outline = '4px solid #ff4444';
    el.style.outlineOffset = '2px';
    setTimeout(() => {
      el.style.outline = originalOutline;
      el.style.transition = originalTransition;
    }, 2000);
  };

  const findElement = (selector) => {
    // 1. Try as a standard CSS selector
    try {
      const el = document.querySelector(selector);
      if (el) return el;
    } catch (e) { }

    // 2. Try finding by text content (Buttons, Links, etc.)
    const text = selector.toLowerCase();
    const candidates = Array.from(document.querySelectorAll('button, a, input[type="button"], input[type="submit"], [role="button"]'));

    let found = candidates.find(el => el.innerText?.toLowerCase().trim() === text || el.value?.toLowerCase().trim() === text);
    if (found) return found;

    found = candidates.find(el => el.innerText?.toLowerCase().includes(text) || el.value?.toLowerCase().includes(text));
    if (found) return found;

    const all = Array.from(document.body.querySelectorAll('*'));
    return all.find(el => el.children.length === 0 && el.innerText?.toLowerCase().trim() === text);
  };

  if (cmd.action === "scroll") {
    window.scrollBy({
      top: cmd.y || 500,
      left: cmd.x || 0,
      behavior: 'smooth'
    });
    return "Scrolled";
  }

  if (cmd.action === "wait") {
    return new Promise(resolve => {
      setTimeout(() => resolve("Wait finished"), cmd.ms || 2000);
    });
  }

  if (cmd.action === "get_elements") {
    const rawItems = Array.from(document.querySelectorAll('button, a, input, select, [role="button"]'))
      .filter(el => {
        const rect = el.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0 && window.getComputedStyle(el).visibility !== 'hidden';
      });

    // Score and filter elements for high-intent
    const items = rawItems
      .map(el => {
        const text = (el.innerText || el.value || el.placeholder || "").trim().substring(0, 50);
        let priority = 1;

        // Priority for navigation and primary actions
        if (text.toLowerCase().includes('next') || text.toLowerCase().includes('prev')) priority = 10;
        if (text.match(/^[0-9]+$/)) priority = 5; // Page numbers
        if (el.tagName === 'BUTTON') priority += 2;
        if (el.role === 'button') priority += 2;

        return {
          tag: el.tagName.toLowerCase(),
          text,
          type: el.type || "",
          id: el.id || "",
          priority
        };
      })
      .filter(item => item.text.length > 0)
      .sort((a, b) => b.priority - a.priority)
      .slice(0, 30); // Cap at 30 most relevant elements

    return JSON.stringify(items);
  }

  if (cmd.action === "click") {
    const el = findElement(cmd.selector);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      highlightElement(el);
      el.click();
      el.focus();
      return "Clicked element: " + (el.innerText || el.value || cmd.selector);
    } else {
      throw new Error("Element not found: " + cmd.selector);
    }
  }
  else if (cmd.action === "type") {
    const el = findElement(cmd.selector);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      highlightElement(el);
      el.value = cmd.text;
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
      return "Typed into element: " + (el.innerText || el.value || cmd.selector);
    } else {
      throw new Error("Element not found: " + cmd.selector);
    }
  }
  else if (cmd.action === "read") {
    try {
      if (typeof Readability === "undefined") {
        throw new Error("Readability library not loaded");
      }

      // We need a clone because Readability can be destructive on the DOM
      const docClone = document.cloneNode(true);
      const reader = new Readability(docClone);
      const article = reader.parse();

      if (article && article.textContent) {
        // Return cleaned text
        return article.textContent.replace(/\s+/g, ' ').trim();
      } else {
        // Fallback if readability fails
        return document.body.innerText.replace(/\s+/g, ' ').trim();
      }
    } catch (e) {
      console.error("Readability error:", e);
      return document.body.innerText.replace(/\s+/g, ' ').trim();
    }
  }
}

// Start connection
connect();
