document.addEventListener('DOMContentLoaded', () => {
    const statusDot = document.getElementById('status-dot');
    const statusText = document.getElementById('status-text');
    const serverUrl = "ws://127.0.0.1:2345";

    function checkStatus() {
        const socket = new WebSocket(serverUrl);

        socket.onopen = () => {
            statusDot.classList.add('online');
            statusDot.classList.remove('offline');
            statusText.innerText = "Connected to Leo Server";
            statusText.classList.add('online-text');
            socket.close();
        };

        socket.onerror = () => {
            statusDot.classList.add('offline');
            statusDot.classList.remove('online');
            statusText.innerText = "Leo Server Offline";
            statusText.classList.remove('online-text');
        };
    }

    // Initial check
    checkStatus();

    // Refresh status on click
    document.getElementById('refresh-btn').addEventListener('click', () => {
        statusText.innerText = "Checking...";
        checkStatus();
    });

    // Action Buttons
    document.getElementById('open-dashboard').addEventListener('click', () => {
        chrome.tabs.create({ url: 'onboarding.html' });
    });
});
