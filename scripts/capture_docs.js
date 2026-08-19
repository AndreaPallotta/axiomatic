const puppeteer = require('puppeteer');
const path = require('path');
const fs = require('fs');

async function capture() {
    const outDir = path.join(__dirname, '..', 'docs', 'screenshots');
    fs.mkdirSync(outDir, { recursive: true });

    const browser = await puppeteer.launch({
        executablePath: 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
        headless: 'new',
        args: [
            '--no-sandbox',
            '--disable-setuid-sandbox',
            '--disable-web-security'
        ],
        defaultViewport: {
            width: 1440,
            height: 900,
            deviceScaleFactor: 2
        }
    });

    const page = await browser.newPage();
    const res = await fetch('http://127.0.0.1:3000/');
    const html = await res.text();

    await page.setContent(html, { waitUntil: 'load' });

    await page.evaluate(() => {
        // Mock realistic verified proof state on canvas
        document.getElementById('stat-discoveries').innerText = '648';
        document.getElementById('stat-premises').innerText = '1,296';
        document.getElementById('nodes-count').innerText = '6 / 86';
        document.getElementById('iter-count').innerText = '145';
        document.getElementById('epochs-val').innerText = '648';
        document.getElementById('loss-val').innerText = '2.214';
        document.getElementById('solve-val').innerText = '82%';
        document.getElementById('solve-val').style.color = '#137333';
        document.getElementById('goal-input').value = '((x + -(x)) + (y * 1)) = (0 + y)';

        // Populate Formal Proof Trace
        const proofList = document.getElementById('proof-container');
        proofList.innerHTML = `
            <div class="proof-step-tile">1. rw [neg_inverse_left] : ((0 + (y * 1)) = (0 + y))</div>
            <div class="proof-step-tile">2. rw [mul_identity_right] : ((0 + y) = (0 + y))</div>
            <div class="proof-step-tile">3. rfl : (0 + y) = (0 + y) &nbsp; [Q.E.D.]</div>
        `;

        // Populate Discoveries Feed
        const tbody = document.getElementById('discoveries-tbody');
        tbody.innerHTML = `
            <tr><td><code>D(((x + -x) + (x * 1))) = (0 + 1)</code></td><td><span class="domain-pill domain-calculus">CALCULUS</span></td><td>7</td><td><button class="btn btn-tonal btn-sm" style="padding:1px 5px; font-size:0.65rem;">Load</button></td></tr>
            <tr><td><code>!(!(!a & !b) & !c) = ((a | b) | c)</code></td><td><span class="domain-pill domain-boolean">BOOLEAN</span></td><td>5</td><td><button class="btn btn-tonal btn-sm" style="padding:1px 5px; font-size:0.65rem;">Load</button></td></tr>
            <tr><td><code>((a & b) | (a & !b)) = (a & 1)</code></td><td><span class="domain-pill domain-boolean">BOOLEAN</span></td><td>4</td><td><button class="btn btn-tonal btn-sm" style="padding:1px 5px; font-size:0.65rem;">Load</button></td></tr>
            <tr><td><code>inter(union(A, B), union(A, comp(B))) = union(A, 0)</code></td><td><span class="domain-pill domain-set_theory">SET THEORY</span></td><td>6</td><td><button class="btn btn-tonal btn-sm" style="padding:1px 5px; font-size:0.65rem;">Load</button></td></tr>
            <tr><td><code>(((x + y) + -y) * (1 + 0)) = (0 + x)</code></td><td><span class="domain-pill domain-algebra">ALGEBRA</span></td><td>5</td><td><button class="btn btn-tonal btn-sm" style="padding:1px 5px; font-size:0.65rem;">Load</button></td></tr>
        `;
        document.getElementById('feed-count-badge').innerText = '648 theorems';

        // Loss History
        lossHistory = [6.55, 5.86, 5.21, 4.41, 3.80, 3.09, 3.40, 2.96, 2.82, 2.56, 2.45, 2.41, 2.51, 2.64, 2.59, 2.51, 2.47, 2.39, 2.36, 2.33, 2.27, 2.21];
        renderLossChart();

        // Populate tree nodes with custom layout
        nodes = [
            { id: 0, parent_id: null, x: 80, y: 260, visit_count: 145, mean_value: 0.82, depth: 0, is_proven: false, is_terminal: false, is_expanded: true, applied_tactic: null },
            { id: 1, parent_id: 0, x: 280, y: 160, visit_count: 98, mean_value: 0.94, depth: 1, is_proven: false, is_terminal: false, is_expanded: true, applied_tactic: { RewriteLhs: 'neg_inverse_left' } },
            { id: 2, parent_id: 0, x: 280, y: 260, visit_count: 22, mean_value: -0.15, depth: 1, is_proven: false, is_terminal: false, is_expanded: true, applied_tactic: { RewriteLhs: 'add_comm' } },
            { id: 3, parent_id: 0, x: 280, y: 360, visit_count: 25, mean_value: -0.05, depth: 1, is_proven: false, is_terminal: false, is_expanded: true, applied_tactic: { RewriteRhs: 'add_identity_left' } },
            { id: 4, parent_id: 1, x: 480, y: 160, visit_count: 85, mean_value: 0.98, depth: 2, is_proven: false, is_terminal: false, is_expanded: true, applied_tactic: { RewriteLhs: 'mul_identity_right' } },
            { id: 5, parent_id: 1, x: 480, y: 80, visit_count: 13, mean_value: 0.10, depth: 2, is_proven: false, is_terminal: false, is_expanded: true, applied_tactic: { RewriteRhs: 'add_comm' } },
            { id: 6, parent_id: 4, x: 680, y: 160, visit_count: 80, mean_value: 1.00, depth: 3, is_proven: true, is_terminal: true, is_expanded: false, applied_tactic: 'rfl' }
        ];

        zoom = 1.25;
        panX = 60;
        panY = 120;
        renderTree(nodes);
    });

    await new Promise(r => setTimeout(r, 600));

    const outPath = path.join(outDir, 'cockpit_live.png');
    await page.screenshot({ path: outPath, type: 'png' });
    console.log(`[SUCCESS] High-res screenshot captured: ${outPath}`);

    await browser.close();
}

capture().catch(err => {
    console.error(err);
    process.exit(1);
});
