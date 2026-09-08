use crate::search::mcts::SearchGraphSnapshot;
use crate::verifier::exporter::MultiFormatExporter;
use crate::verifier::kernel::ProofState;
use crate::verifier::lean_runner::{Lean4Validator, LeanValidationResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticProofTrace {
    pub title: String,
    pub conjecture: String,
    pub domain: String,
    pub is_proven: bool,
    pub total_nodes: usize,
    pub total_iterations: usize,
    pub proven_node_id: Option<usize>,
    pub proof_path_node_ids: Vec<usize>,
    pub tactics_sequence: Vec<String>,
    pub lean4_code: Option<String>,
    pub coq_code: Option<String>,
    pub latex_code: Option<String>,
    pub lean4_certified: bool,
    pub lean4_kernel_time_ms: Option<f64>,
    pub lean4_version: Option<String>,
    pub graph_snapshot: SearchGraphSnapshot,
    pub timestamp: String,
}

pub struct TraceExporter;

impl TraceExporter {
    pub fn create_trace(
        title: &str,
        conjecture_str: &str,
        domain_str: &str,
        snapshot: SearchGraphSnapshot,
    ) -> StaticProofTrace {
        let is_proven = snapshot.proven_node_id.is_some();
        let total_nodes = snapshot.nodes.len();
        let total_iterations = snapshot.total_iterations;
        let proven_node_id = snapshot.proven_node_id;

        let mut proof_path_node_ids = Vec::new();
        if let Some(mut curr) = proven_node_id {
            proof_path_node_ids.push(curr);
            while let Some(parent) = snapshot.nodes.get(curr).and_then(|n| n.parent_id) {
                proof_path_node_ids.push(parent);
                curr = parent;
            }
            proof_path_node_ids.reverse();
        }

        let solved_state: Option<&ProofState> = proven_node_id
            .and_then(|id| snapshot.nodes.get(id))
            .map(|n| &n.state);

        let tactics_sequence: Vec<String> = solved_state
            .map(|s| s.proof_history.iter().map(|step| step.1.clone()).collect())
            .unwrap_or_default();

        let (lean4_code, coq_code, latex_code) = if let Some(state) = solved_state {
            (
                Some(MultiFormatExporter::to_lean4(title, state)),
                Some(MultiFormatExporter::to_coq(title, state)),
                Some(MultiFormatExporter::to_latex(title, state)),
            )
        } else {
            (None, None, None)
        };

        let (lean4_certified, lean4_kernel_time_ms, lean4_version) =
            if let Some(state) = solved_state {
                match Lean4Validator::validate_proof(title, state) {
                    LeanValidationResult::Certified {
                        elapsed_ms,
                        lean_version,
                    } => (true, Some(elapsed_ms), Some(lean_version)),
                    _ => (false, None, None),
                }
            } else {
                (false, None, None)
            };

        let timestamp = chrono::Utc::now().to_rfc3339();

        StaticProofTrace {
            title: title.to_string(),
            conjecture: conjecture_str.to_string(),
            domain: domain_str.to_string(),
            is_proven,
            total_nodes,
            total_iterations,
            proven_node_id,
            proof_path_node_ids,
            tactics_sequence,
            lean4_code,
            coq_code,
            latex_code,
            lean4_certified,
            lean4_kernel_time_ms,
            lean4_version,
            graph_snapshot: snapshot,
            timestamp,
        }
    }

    pub fn export_json(trace: &StaticProofTrace, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(trace)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        fs::write(path, serialized)
    }

    pub fn export_standalone_html(trace: &StaticProofTrace, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json_payload = serde_json::to_string(trace)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let html_content = generate_html_showcase(&json_payload, trace);
        fs::write(path, html_content)
    }
}

fn generate_html_showcase(json_data: &str, trace: &StaticProofTrace) -> String {
    let sanitized_json = json_data.replace("</script>", "<\\/script>");
    let status_badge_class = if trace.is_proven {
        "badge-success"
    } else {
        "badge-warning"
    };
    let status_text = if trace.is_proven {
        "PROVEN"
    } else {
        "UNRESOLVED"
    };
    let certified_badge = if trace.lean4_certified {
        format!(
            "<span class='badge badge-success'>Lean 4 Certified ({:.1} ms)</span>",
            trace.lean4_kernel_time_ms.unwrap_or(0.0)
        )
    } else {
        "<span class='badge badge-muted'>Uncertified</span>".to_string()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Axiomatic Proof Showcase - {title}</title>
    <style>
        :root {{
            --bg-color: #0d1117;
            --surface-color: #161b22;
            --surface-border: #30363d;
            --text-main: #f0f6fc;
            --text-secondary: #8b949e;
            --accent-primary: #58a6ff;
            --accent-success: #3fb950;
            --accent-warning: #d29922;
            --accent-danger: #f85149;
            --font-code: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, Courier, monospace;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background-color: var(--bg-color);
            color: var(--text-main);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
            display: flex;
            flex-direction: column;
            height: 100vh;
            overflow: hidden;
        }}
        header {{
            background: var(--surface-color);
            border-bottom: 1px solid var(--surface-border);
            padding: 12px 24px;
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 16px;
        }}
        .brand {{
            display: flex;
            align-items: center;
            gap: 12px;
        }}
        .brand-logo {{
            width: 32px;
            height: 32px;
            border-radius: 8px;
            background: linear-gradient(135deg, #1f6feb, #238636);
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: bold;
            font-size: 18px;
            color: #ffffff;
        }}
        .brand h1 {{
            font-size: 16px;
            font-weight: 600;
        }}
        .brand p {{
            font-size: 12px;
            color: var(--text-secondary);
            font-family: var(--font-code);
        }}
        .badges {{
            display: flex;
            align-items: center;
            gap: 8px;
        }}
        .badge {{
            padding: 4px 10px;
            border-radius: 12px;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}
        .badge-success {{ background: rgba(63, 185, 80, 0.2); color: var(--accent-success); border: 1px solid var(--accent-success); }}
        .badge-warning {{ background: rgba(210, 153, 34, 0.2); color: var(--accent-warning); border: 1px solid var(--accent-warning); }}
        .badge-muted {{ background: rgba(139, 148, 158, 0.2); color: var(--text-secondary); border: 1px solid var(--surface-border); }}
        .main-layout {{
            display: flex;
            flex: 1;
            height: calc(100vh - 60px);
            overflow: hidden;
        }}
        .canvas-container {{
            flex: 1;
            position: relative;
            background: radial-gradient(circle at center, #161b22 0%, #0d1117 100%);
            overflow: hidden;
        }}
        canvas {{
            display: block;
            cursor: grab;
            width: 100%;
            height: 100%;
        }}
        canvas:active {{
            cursor: grabbing;
        }}
        .tree-controls {{
            position: absolute;
            bottom: 20px;
            left: 20px;
            display: flex;
            gap: 8px;
            background: var(--surface-color);
            padding: 6px;
            border-radius: 8px;
            border: 1px solid var(--surface-border);
            z-index: 10;
        }}
        .ctrl-btn {{
            background: transparent;
            border: 1px solid var(--surface-border);
            color: var(--text-main);
            padding: 6px 12px;
            border-radius: 6px;
            cursor: pointer;
            font-size: 12px;
            font-weight: 500;
        }}
        .ctrl-btn:hover {{
            background: var(--surface-border);
        }}
        .sidebar {{
            width: 420px;
            background: var(--surface-color);
            border-left: 1px solid var(--surface-border);
            display: flex;
            flex-direction: column;
            overflow: hidden;
        }}
        .tabs {{
            display: flex;
            border-bottom: 1px solid var(--surface-border);
            background: #0d1117;
        }}
        .tab-btn {{
            flex: 1;
            padding: 10px 8px;
            background: transparent;
            border: none;
            color: var(--text-secondary);
            font-size: 12px;
            font-weight: 600;
            cursor: pointer;
            border-bottom: 2px solid transparent;
        }}
        .tab-btn.active {{
            color: var(--accent-primary);
            border-bottom-color: var(--accent-primary);
            background: var(--surface-color);
        }}
        .tab-content {{
            display: none;
            flex: 1;
            overflow-y: auto;
            padding: 16px;
        }}
        .tab-content.active {{
            display: block;
        }}
        .card {{
            background: #0d1117;
            border: 1px solid var(--surface-border);
            border-radius: 8px;
            padding: 14px;
            margin-bottom: 14px;
        }}
        .card h3 {{
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--text-secondary);
            margin-bottom: 8px;
        }}
        .stat-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 10px;
        }}
        .stat-item {{
            display: flex;
            flex-direction: column;
            gap: 2px;
        }}
        .stat-label {{
            font-size: 11px;
            color: var(--text-secondary);
        }}
        .stat-value {{
            font-size: 14px;
            font-weight: 600;
            color: var(--text-main);
            font-family: var(--font-code);
        }}
        pre.code-block {{
            background: #090d13;
            border: 1px solid var(--surface-border);
            border-radius: 6px;
            padding: 12px;
            font-family: var(--font-code);
            font-size: 12px;
            line-height: 1.5;
            color: #c9d1d9;
            overflow-x: auto;
            white-space: pre;
        }}
        .copy-btn {{
            float: right;
            background: var(--surface-border);
            border: none;
            color: var(--text-main);
            font-size: 11px;
            padding: 3px 8px;
            border-radius: 4px;
            cursor: pointer;
        }}
        .copy-btn:hover {{
            background: #484f58;
        }}
        .path-list {{
            list-style: none;
        }}
        .path-item {{
            padding: 8px 10px;
            border-left: 2px solid var(--surface-border);
            margin-bottom: 6px;
            font-size: 12px;
            font-family: var(--font-code);
            background: #090d13;
            border-radius: 0 6px 6px 0;
            cursor: pointer;
        }}
        .path-item:hover, .path-item.active {{
            border-left-color: var(--accent-success);
            background: #111b27;
        }}
    </style>
</head>
<body>
    <header>
        <div class="brand">
            <div class="brand-logo">&Sigma;</div>
            <div>
                <h1>Axiomatic // Proof Showcase</h1>
                <p>{conjecture}</p>
            </div>
        </div>
        <div class="badges">
            <span class="badge {status_badge_class}">{status_text}</span>
            {certified_badge}
        </div>
    </header>

    <div class="main-layout">
        <div class="canvas-container">
            <canvas id="mctsCanvas"></canvas>
            <div class="tree-controls">
                <button class="ctrl-btn" id="btnZoomIn">Zoom +</button>
                <button class="ctrl-btn" id="btnZoomOut">Zoom -</button>
                <button class="ctrl-btn" id="btnResetView">Reset View</button>
                <button class="ctrl-btn" id="btnCenterProven">Focus Proven Path</button>
            </div>
        </div>

        <div class="sidebar">
            <div class="tabs">
                <button class="tab-btn active" data-tab="tab-inspect">Inspector</button>
                <button class="tab-btn" data-tab="tab-lean">Lean 4</button>
                <button class="tab-btn" data-tab="tab-coq">Coq</button>
                <button class="tab-btn" data-tab="tab-latex">LaTeX</button>
            </div>

            <div id="tab-inspect" class="tab-content active">
                <div class="card">
                    <h3>Search Summary</h3>
                    <div class="stat-grid">
                        <div class="stat-item">
                            <span class="stat-label">Domain</span>
                            <span class="stat-value">{domain}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">MCTS Nodes</span>
                            <span class="stat-value">{total_nodes}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">Iterations</span>
                            <span class="stat-value">{total_iterations}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">Proof Steps</span>
                            <span class="stat-value">{proof_steps_count}</span>
                        </div>
                    </div>
                </div>

                <div class="card">
                    <h3>Selected Node Details</h3>
                    <div id="selectedNodeDetails">
                        <p style="font-size:12px; color:var(--text-secondary);">Click any node in the tree to inspect proof state, neural priors, and Q-values.</p>
                    </div>
                </div>

                <div class="card">
                    <h3>Proven Tactic Sequence</h3>
                    <ul class="path-list" id="tacticsList"></ul>
                </div>
            </div>

            <div id="tab-lean" class="tab-content">
                <div class="card">
                    <button class="copy-btn" onclick="copyCode('leanCode')">Copy</button>
                    <h3>Lean 4 Formal Proof</h3>
                    <pre class="code-block" id="leanCode"></pre>
                </div>
            </div>

            <div id="tab-coq" class="tab-content">
                <div class="card">
                    <button class="copy-btn" onclick="copyCode('coqCode')">Copy</button>
                    <h3>Coq / Rocq Specification</h3>
                    <pre class="code-block" id="coqCode"></pre>
                </div>
            </div>

            <div id="tab-latex" class="tab-content">
                <div class="card">
                    <button class="copy-btn" onclick="copyCode('latexCode')">Copy</button>
                    <h3>AMS-LaTeX Derivation</h3>
                    <pre class="code-block" id="latexCode"></pre>
                </div>
            </div>
        </div>
    </div>

    <script id="traceData" type="application/json">
{json_payload}
    </script>

    <script>
        const trace = JSON.parse(document.getElementById('traceData').textContent);
        const canvas = document.getElementById('mctsCanvas');
        const ctx = canvas.getContext('2d');

        let panX = 100;
        let panY = 150;
        let zoom = 1.0;
        let isDragging = false;
        let startX = 0, startY = 0;
        let selectedNodeId = trace.proven_node_id !== null ? trace.proven_node_id : 0;

        const provenPathSet = new Set(trace.proof_path_node_ids || []);
        const nodes = trace.graph_snapshot.nodes || [];

        function resize() {{
            canvas.width = canvas.parentElement.clientWidth;
            canvas.height = canvas.parentElement.clientHeight;
            render();
        }}
        window.addEventListener('resize', resize);

        document.querySelectorAll('.tab-btn').forEach(btn => {{
            btn.addEventListener('click', () => {{
                document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
                document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
                btn.classList.add('active');
                document.getElementById(btn.dataset.tab).classList.add('active');
            }});
        }});

        document.getElementById('leanCode').textContent = trace.lean4_code || '-- No Lean 4 code generated';
        document.getElementById('coqCode').textContent = trace.coq_code || '(* No Coq code generated *)';
        document.getElementById('latexCode').textContent = trace.latex_code || '% No LaTeX code generated';

        function populateTactics() {{
            const list = document.getElementById('tacticsList');
            list.innerHTML = '';
            if (!trace.tactics_sequence || trace.tactics_sequence.length === 0) {{
                list.innerHTML = '<li style="color:var(--text-secondary); font-size:12px;">No proof steps found</li>';
                return;
            }}
            trace.tactics_sequence.forEach((tactic, idx) => {{
                const li = document.createElement('li');
                li.className = 'path-item';
                li.textContent = `${{idx + 1}}. ${{tactic}}`;
                list.appendChild(li);
            }});
        }}
        populateTactics();

        function computeTreeLayout() {{
            const levels = [];
            nodes.forEach(node => {{
                const d = node.depth || 0;
                while (levels.length <= d) levels.push([]);
                levels[d].push(node);
            }});

            const xSpacing = 160;
            const ySpacing = 65;

            levels.forEach((levelNodes, depth) => {{
                const totalHeight = (levelNodes.length - 1) * ySpacing;
                levelNodes.forEach((node, idx) => {{
                    node.x = depth * xSpacing + 60;
                    node.y = (idx * ySpacing) - (totalHeight / 2) + 200;
                }});
            }});
        }}
        computeTreeLayout();

        function render() {{
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            ctx.save();
            ctx.translate(panX, panY);
            ctx.scale(zoom, zoom);

            const nodeMap = new Map();
            nodes.forEach(n => nodeMap.set(n.id, n));

            nodes.forEach(node => {{
                if (node.parent_id !== null && nodeMap.has(node.parent_id)) {{
                    const parent = nodeMap.get(node.parent_id);
                    const isProvenEdge = provenPathSet.has(node.id) && provenPathSet.has(parent.id);

                    ctx.beginPath();
                    ctx.strokeStyle = isProvenEdge ? '#3fb950' : '#30363d';
                    ctx.lineWidth = isProvenEdge ? 3.0 : 1.5;

                    const midX = (parent.x + node.x) / 2;
                    ctx.moveTo(parent.x, parent.y);
                    ctx.bezierCurveTo(midX, parent.y, midX, node.y, node.x, node.y);
                    ctx.stroke();

                    if (node.applied_tactic) {{
                        const tacticStr = typeof node.applied_tactic === 'string'
                            ? node.applied_tactic
                            : (node.applied_tactic.RewriteLhs ? `rw [${{node.applied_tactic.RewriteLhs}}]` : (node.applied_tactic.RewriteRhs ? `nth_rw [${{node.applied_tactic.RewriteRhs}}]` : JSON.stringify(node.applied_tactic)));

                        const labelX = (parent.x + node.x) / 2;
                        const labelY = (parent.y + node.y) / 2 - 8;

                        ctx.font = '500 10px "SFMono-Regular", Consolas, monospace';
                        const textWidth = ctx.measureText(tacticStr).width;

                        ctx.fillStyle = isProvenEdge ? '#112211' : '#161b22';
                        ctx.fillRect(labelX - textWidth / 2 - 4, labelY - 7, textWidth + 8, 14);
                        ctx.strokeStyle = isProvenEdge ? '#2ea043' : '#30363d';
                        ctx.lineWidth = 1;
                        ctx.strokeRect(labelX - textWidth / 2 - 4, labelY - 7, textWidth + 8, 14);

                        ctx.fillStyle = isProvenEdge ? '#3fb950' : '#8b949e';
                        ctx.textAlign = 'center';
                        ctx.textBaseline = 'middle';
                        ctx.fillText(tacticStr, labelX, labelY);
                    }}
                }}
            }});

            nodes.forEach(node => {{
                ctx.beginPath();
                const radius = Math.min(22, Math.max(14, 14 + Math.log2(node.visit_count + 1) * 2.2));
                ctx.arc(node.x, node.y, radius, 0, 2 * Math.PI);

                const isProven = node.is_proven || provenPathSet.has(node.id);
                const isSelected = node.id === selectedNodeId;

                if (isProven) {{
                    ctx.fillStyle = '#238636';
                    ctx.strokeStyle = '#3fb950';
                    ctx.lineWidth = 2.5;
                }} else if (isSelected) {{
                    ctx.fillStyle = '#1f6feb';
                    ctx.strokeStyle = '#58a6ff';
                    ctx.lineWidth = 2.5;
                }} else if (node.id === 0) {{
                    ctx.fillStyle = '#9e6a03';
                    ctx.strokeStyle = '#d29922';
                    ctx.lineWidth = 2;
                }} else {{
                    ctx.fillStyle = '#21262d';
                    ctx.strokeStyle = '#30363d';
                    ctx.lineWidth = 1.5;
                }}

                ctx.fill();
                ctx.stroke();

                ctx.fillStyle = '#ffffff';
                ctx.font = '600 10px "SFMono-Regular", Consolas, monospace';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.fillText(`N:${{node.visit_count}}`, node.x, node.y);
            }});

            ctx.restore();
        }}

        function updateInspector(node) {{
            if (!node) return;
            const container = document.getElementById('selectedNodeDetails');
            const state = node.state || {{}};
            const openGoals = state.open_goals || [];
            const goalsHtml = openGoals.length === 0
                ? '<span style="color:var(--accent-success); font-weight:600;">Reflexivity / All Goals Solved</span>'
                : openGoals.map(g => `<code style="color:#58a6ff;">${{g.equality ? `${{g.equality.lhs}} = ${{g.equality.rhs}}` : JSON.stringify(g)}}</code>`).join('<br>');

            container.innerHTML = `
                <div class="stat-grid" style="margin-bottom:12px;">
                    <div class="stat-item">
                        <span class="stat-label">Node ID</span>
                        <span class="stat-value">#${{node.id}}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Depth</span>
                        <span class="stat-value">${{node.depth}}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Visits N(s)</span>
                        <span class="stat-value">${{node.visit_count}}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Mean Q(s)</span>
                        <span class="stat-value">${{node.mean_value.toFixed(4)}}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Policy Prior P(s,a)</span>
                        <span class="stat-value">${{node.policy_prior.toFixed(4)}}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Status</span>
                        <span class="stat-value" style="color:${{node.is_proven ? 'var(--accent-success)' : 'var(--text-secondary)'}};">
                            ${{node.is_proven ? 'Proven' : (node.is_terminal ? 'Terminal' : 'Open')}}
                        </span>
                    </div>
                </div>
                <div style="font-size:11px; color:var(--text-secondary); margin-bottom:4px;">Open Goals:</div>
                <div style="background:#090d13; padding:8px; border-radius:6px; font-size:12px; font-family:var(--font-code);">${{goalsHtml}}</div>
            `;
        }}

        canvas.addEventListener('mousedown', e => {{
            isDragging = true;
            startX = e.clientX - panX;
            startY = e.clientY - panY;
        }});
        window.addEventListener('mouseup', () => isDragging = false);
        window.addEventListener('mousemove', e => {{
            if (isDragging) {{
                panX = e.clientX - startX;
                panY = e.clientY - startY;
                render();
            }}
        }});
        canvas.addEventListener('wheel', e => {{
            e.preventDefault();
            const mouseX = e.clientX - canvas.getBoundingClientRect().left;
            const mouseY = e.clientY - canvas.getBoundingClientRect().top;
            const factor = e.deltaY < 0 ? 1.15 : 0.85;
            const newZoom = Math.max(0.15, Math.min(3.0, zoom * factor));
            panX = mouseX - (mouseX - panX) * (newZoom / zoom);
            panY = mouseY - (mouseY - panY) * (newZoom / zoom);
            zoom = newZoom;
            render();
        }});

        canvas.addEventListener('click', e => {{
            const rect = canvas.getBoundingClientRect();
            const mouseX = (e.clientX - rect.left - panX) / zoom;
            const mouseY = (e.clientY - rect.top - panY) / zoom;

            let closest = null;
            let minDist = 25;
            nodes.forEach(n => {{
                const dist = Math.hypot(n.x - mouseX, n.y - mouseY);
                if (dist < minDist) {{
                    minDist = dist;
                    closest = n;
                }}
            }});

            if (closest) {{
                selectedNodeId = closest.id;
                updateInspector(closest);
                render();
            }}
        }});

        document.getElementById('btnZoomIn').onclick = () => {{ zoom = Math.min(3.0, zoom * 1.25); render(); }};
        document.getElementById('btnZoomOut').onclick = () => {{ zoom = Math.max(0.15, zoom * 0.8); render(); }};
        document.getElementById('btnResetView').onclick = () => {{ panX = 100; panY = 150; zoom = 1.0; render(); }};
        document.getElementById('btnCenterProven').onclick = () => {{
            if (trace.proven_node_id !== null) {{
                const pNode = nodes.find(n => n.id === trace.proven_node_id);
                if (pNode) {{
                    panX = canvas.width / 2 - pNode.x * zoom;
                    panY = canvas.height / 2 - pNode.y * zoom;
                    selectedNodeId = pNode.id;
                    updateInspector(pNode);
                    render();
                }}
            }}
        }};

        function copyCode(id) {{
            const text = document.getElementById(id).textContent;
            navigator.clipboard.writeText(text);
        }}

        resize();
        if (nodes.length > 0) {{
            const initNode = nodes.find(n => n.id === selectedNodeId) || nodes[0];
            updateInspector(initNode);
        }}
    </script>
</body>
</html>
"#,
        title = trace.title,
        conjecture = trace.conjecture,
        status_badge_class = status_badge_class,
        status_text = status_text,
        certified_badge = certified_badge,
        domain = trace.domain,
        total_nodes = trace.total_nodes,
        total_iterations = trace.total_iterations,
        proof_steps_count = trace.tactics_sequence.len(),
        json_payload = sanitized_json
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::fol::{Equality, Term};
    use crate::verifier::kernel::{ProofState, Tactic};

    #[test]
    fn test_create_and_export_trace() -> Result<(), Box<dyn std::error::Error>> {
        let x = Term::var("x");
        let zero = Term::constant("0");
        let goal = Equality::new(Term::func("+", vec![x.clone(), zero.clone()]), x.clone());
        let mut state = ProofState::new(goal);

        state.proof_history.push((
            Tactic::RewriteLhs("add_zero".to_string()),
            "Rewrote LHS via [add_zero]: x = x".to_string(),
        ));
        state
            .proof_history
            .push((Tactic::Reflexivity, "Solved #1: x = x via rfl".to_string()));
        state.is_solved = true;

        let root_node = crate::search::node::MctsNode::new_root(state.clone());
        let child_node = crate::search::node::MctsNode::new_child(
            1,
            0,
            state.clone(),
            Tactic::Reflexivity,
            0.9,
            1,
        );

        let snapshot = SearchGraphSnapshot {
            nodes: vec![root_node, child_node],
            total_iterations: 10,
            proven_node_id: Some(1),
        };

        let trace = TraceExporter::create_trace("test_theorem", "(x + 0) = x", "Algebra", snapshot);

        assert_eq!(trace.title, "test_theorem");
        assert!(trace.is_proven);
        assert_eq!(trace.proven_node_id, Some(1));
        assert_eq!(trace.proof_path_node_ids, vec![0, 1]);
        assert_eq!(trace.tactics_sequence.len(), 2);
        assert!(trace.lean4_code.is_some());

        let temp_dir = std::env::temp_dir().join("axiomatic_trace_test");
        let json_path = temp_dir.join("trace.json");
        let html_path = temp_dir.join("index.html");

        TraceExporter::export_json(&trace, &json_path)?;
        assert!(json_path.exists());
        let json_str = fs::read_to_string(&json_path)?;
        assert!(json_str.contains("test_theorem"));

        TraceExporter::export_standalone_html(&trace, &html_path)?;
        assert!(html_path.exists());
        let html_str = fs::read_to_string(&html_path)?;
        assert!(html_str.contains("<!DOCTYPE html>"));
        assert!(html_str.contains("test_theorem"));

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
