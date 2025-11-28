import puppeteer from 'puppeteer-core';

const HOST_IP = '172.29.160.1'; // Corrected host IP from ip route
const PORT = 9222;
const TARGET_URL = 'http://127.0.0.1:11391/static/v2/index.html';

async function debugRemote() {
    console.log(`Attempting to connect to Host Chrome at ${HOST_IP}:${PORT}...`);

    try {
        const browser = await puppeteer.connect({
            browserURL: `http://${HOST_IP}:${PORT}`,
            defaultViewport: null
        });

        console.log('Connected to Host Chrome!');

        const page = await browser.newPage();

        // Capture logs
        page.on('console', msg => console.log('BROWSER CONSOLE:', msg.type().toUpperCase(), msg.text()));
        page.on('pageerror', err => console.error('BROWSER ERROR:', err.toString()));
        page.on('requestfailed', request => console.error('REQUEST FAILED:', request.url(), request.failure().errorText));

        console.log(`Navigating to ${TARGET_URL}...`);
        await page.goto(TARGET_URL);

        // Wait a bit for errors to appear
        await new Promise(r => setTimeout(r, 3000));

        // Take screenshot
        await page.screenshot({ path: 'debug_screenshot.png' });
        console.log('Screenshot saved to debug_screenshot.png');

        await browser.disconnect();
    } catch (e) {
        console.error('CONNECTION FAILED:', e.message);
        console.log('\nPLEASE ENSURE YOU STARTED CHROME ON WINDOWS WITH:');
        console.log(`chrome.exe --remote-debugging-port=${PORT} --remote-allow-origins=* --remote-debugging-address=0.0.0.0`);
    }
}

debugRemote();
