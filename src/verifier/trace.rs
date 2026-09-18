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
        r##"<!DOCTYPE html>
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
        .demo-nav {{
            display: flex;
            align-items: center;
            gap: 6px;
        }}
        .demo-label {{
            font-size: 11px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--text-secondary);
            margin-right: 4px;
        }}
        .demo-chip {{
            padding: 4px 10px;
            border-radius: 6px;
            font-size: 11px;
            font-weight: 500;
            text-decoration: none;
            color: var(--text-secondary);
            background: #0d1117;
            border: 1px solid var(--surface-border);
            transition: all 0.15s ease;
        }}
        .demo-chip:hover {{
            color: var(--text-main);
            border-color: var(--accent-primary);
        }}
        .demo-chip.active {{
            color: var(--accent-primary);
            border-color: var(--accent-primary);
            background: rgba(88, 166, 255, 0.1);
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
        .ctrl-btn.active {{
            background: var(--accent-primary);
            color: #ffffff;
            border-color: var(--accent-primary);
        }}
        #galaxyHud {{
            position: absolute;
            top: 16px;
            left: 16px;
            pointer-events: none;
            display: flex;
            flex-direction: column;
            gap: 8px;
            font-family: var(--font-code);
            z-index: 10;
        }}
        .hud-card {{
            background: rgba(13, 17, 23, 0.85);
            backdrop-filter: blur(8px);
            border: 1px solid var(--surface-border);
            border-radius: 6px;
            padding: 6px 12px;
            font-size: 11px;
            display: flex;
            align-items: center;
            gap: 12px;
        }}
        #galaxyTooltip {{
            display: none;
            position: absolute;
            pointer-events: none;
            background: rgba(22, 27, 34, 0.95);
            backdrop-filter: blur(10px);
            border: 1px solid var(--surface-border);
            border-radius: 8px;
            padding: 10px 14px;
            color: var(--text-main);
            font-size: 12px;
            max-width: 320px;
            box-shadow: 0 8px 24px rgba(0,0,0,0.6);
            z-index: 20;
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
        <nav class="demo-nav">
            <span class="demo-label">Showcases:</span>
            <a id="demoLinkMain" href="./" class="demo-chip">Coupled Harmonic ODE (1,301 Nodes)</a>
            <a id="demoLinkCalc" href="./calculus/" class="demo-chip">Harmonic Oscillator (500 Nodes)</a>
            <a id="demoLinkMulti" href="./algebra/" class="demo-chip">Multi-Variable Ring (1,937 Nodes)</a>
            <a id="demoLinkBool" href="./bool/" class="demo-chip">Boolean Logic</a>
        </nav>
        <div class="badges">
            <span class="badge {status_badge_class}">{status_text}</span>
            {certified_badge}
            <a href="https://github.com/AndreaPallotta/axiomatic" target="_blank" rel="noopener" class="badge badge-muted" style="text-decoration:none;">GitHub &nearr;</a>
        </div>
    </header>

    <div class="main-layout">
        <div class="canvas-container">
            <canvas id="mctsCanvas"></canvas>
            <div id="galaxyContainer" style="display:none; position:absolute; top:0; left:0; width:100%; height:100%; overflow:hidden;">
                <canvas id="galaxyCanvas" style="width:100%; height:100%; cursor:grab; background:radial-gradient(ellipse at center, #111927 0%, #080b11 100%);"></canvas>
                <div id="galaxyHud">
                    <div class="hud-card">
                        <span>Nodes: <b id="galaxyNodesCount" style="color:var(--accent-primary);">0</b></span>
                        <span>Trajectory: <b id="galaxyTrajCount" style="color:var(--accent-success);">0</b></span>
                        <span>Yaw: <b id="galaxyYawVal" style="color:#c9d1d9;">-31&deg;</b></span>
                        <span>Pitch: <b id="galaxyPitchVal" style="color:#c9d1d9;">20&deg;</b></span>
                    </div>
                    <div class="hud-card" style="font-size:10px; color:var(--text-secondary); gap:8px;">
                        <span style="display:flex; align-items:center; gap:3px;"><span style="width:7px; height:7px; border-radius:50%; background:#2979ff; display:inline-block;"></span> Algebra</span>
                        <span style="display:flex; align-items:center; gap:3px;"><span style="width:7px; height:7px; border-radius:50%; background:#b060ff; display:inline-block;"></span> Boolean</span>
                        <span style="display:flex; align-items:center; gap:3px;"><span style="width:7px; height:7px; border-radius:50%; background:#ff9800; display:inline-block;"></span> Calculus</span>
                        <span style="display:flex; align-items:center; gap:3px;"><span style="width:7px; height:7px; border-radius:50%; background:#26a69a; display:inline-block;"></span> Set Theory</span>
                        <span style="display:flex; align-items:center; gap:3px;"><span style="width:7px; height:7px; border-radius:50%; background:#00bcd4; display:inline-block;"></span> LinAlg</span>
                    </div>
                </div>
                <div id="galaxyTooltip">
                    <div id="galaxyTtTitle" style="font-weight:700; font-size:13px; color:var(--accent-primary); font-family:var(--font-code);"></div>
                    <div id="galaxyTtDomain" style="font-size:11px; color:var(--text-secondary); margin-top:2px;"></div>
                    <div id="galaxyTtStmt" style="font-family:var(--font-code); font-size:12px; color:var(--text-main); margin-top:6px; word-break:break-all;"></div>
                    <div id="galaxyTtTactic" style="font-size:11px; color:var(--accent-success); margin-top:4px; font-family:var(--font-code);"></div>
                </div>
            </div>
            <div class="tree-controls">
                <div style="display:flex; gap:4px; margin-right:8px; border-right:1px solid var(--surface-border); padding-right:8px;">
                    <button class="ctrl-btn active" id="btnView2D" onclick="switchView('2d')">2D Tree</button>
                    <button class="ctrl-btn" id="btnView3D" onclick="switchView('3d')">3D Galaxy</button>
                </div>
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
                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px;">
                        <h3 style="margin-bottom:0;">Equation AST Inspector</h3>
                        <span id="astTargetName" style="font-size:11px; font-weight:700; color:var(--accent-primary); font-family:var(--font-code);">Goal</span>
                    </div>
                    <div id="astEquationExpr" style="font-family:var(--font-code); font-size:11px; background:#090d13; padding:6px 10px; border-radius:6px; border:1px solid var(--surface-border); word-break:break-all; font-weight:600; color:var(--text-main); margin-bottom:8px;"></div>
                    <div id="astSvgContainer" style="width:100%; height:160px; overflow:auto; background:#090d13; border:1px solid var(--surface-border); border-radius:6px; position:relative;"></div>
                    <div style="display:flex; justify-content:space-between; align-items:center; font-size:11px; color:var(--text-secondary); margin-top:6px;">
                        <span style="display:flex; align-items:center; gap:4px;">
                            <span style="width:8px; height:8px; border-radius:50%; background:#58a6ff; display:inline-block;"></span> Op
                            <span style="width:8px; height:8px; border-radius:3px; background:#161b22; border:1px solid #30363d; display:inline-block; margin-left:4px;"></span> Term
                            <span style="width:8px; height:8px; border-radius:50%; background:#e37400; display:inline-block; margin-left:4px;"></span> Rewrite
                        </span>
                        <span id="astNodeCount" style="font-weight:600; font-family:var(--font-code);">0 nodes</span>
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
        const galaxyCanvas = document.getElementById('galaxyCanvas');
        const galaxyCtx = galaxyCanvas ? galaxyCanvas.getContext('2d') : null;

        let currentView = '2d';
        let vectorSpace3D = {{ points: [], trajectory: [], total_nodes: 0 }};
        let galaxyRotX = 0.35;
        let galaxyRotY = -0.55;
        let galaxyCamDist = 340;
        let galaxyPanX = 0;
        let galaxyPanY = 0;
        let isGalaxyDragging = false;
        let galaxyDragStartX = 0;
        let galaxyDragStartY = 0;
        let isGalaxyPanDrag = false;
        let hoveredPoint3D = null;
        let galaxyProjectedItems = [];
        let hasInitialized3D = false;

        let panX = 100;
        let panY = 150;
        let zoom = 1.0;
        let isDragging = false;
        let startX = 0, startY = 0;
        let selectedNodeId = trace.proven_node_id !== null ? trace.proven_node_id : 0;

        const provenPathSet = new Set(trace.proof_path_node_ids || []);
        const nodes = trace.graph_snapshot.nodes || [];

        function switchView(mode) {{
            currentView = mode;
            const btn2d = document.getElementById('btnView2D');
            const btn3d = document.getElementById('btnView3D');
            const mctsEl = document.getElementById('mctsCanvas');
            const galaxyEl = document.getElementById('galaxyContainer');

            if (mode === '2d') {{
                if (btn2d) btn2d.classList.add('active');
                if (btn3d) btn3d.classList.remove('active');
                if (mctsEl) mctsEl.style.display = 'block';
                if (galaxyEl) galaxyEl.style.display = 'none';
                render();
            }} else {{
                if (btn3d) btn3d.classList.add('active');
                if (btn2d) btn2d.classList.remove('active');
                if (mctsEl) mctsEl.style.display = 'none';
                if (galaxyEl) galaxyEl.style.display = 'block';
                if (!hasInitialized3D) {{
                    initVectorSpace3D();
                    hasInitialized3D = true;
                }}
                resizeGalaxy();
                renderGalaxy();
            }}
        }}

        function resize() {{
            canvas.width = canvas.parentElement.clientWidth;
            canvas.height = canvas.parentElement.clientHeight;
            render();
            if (galaxyCanvas && galaxyCanvas.parentElement) {{
                resizeGalaxy();
                if (currentView === '3d') renderGalaxy();
            }}
        }}

        function resizeGalaxy() {{
            if (!galaxyCanvas || !galaxyCanvas.parentElement) return;
            galaxyCanvas.width = galaxyCanvas.parentElement.clientWidth;
            galaxyCanvas.height = galaxyCanvas.parentElement.clientHeight;
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

        const nodeMap = new Map();
        nodes.forEach(n => nodeMap.set(n.id, n));

        const primaryPathSet = new Set(trace.proof_path_node_ids || []);
        const primaryEdges = new Set();
        for (let i = 0; i < (trace.proof_path_node_ids || []).length - 1; i++) {{
            primaryEdges.add(`${{trace.proof_path_node_ids[i]}}->${{trace.proof_path_node_ids[i + 1]}}`);
        }}

        const allProvenPathNodes = new Set();
        const allProvenEdges = new Set();
        nodes.forEach(n => {{
            if (n.is_proven) {{
                let curr = n.id;
                while (curr !== null && curr !== undefined && nodeMap.has(curr)) {{
                    allProvenPathNodes.add(curr);
                    const pId = nodeMap.get(curr).parent_id;
                    if (pId !== null && pId !== undefined) {{
                        allProvenEdges.add(`${{pId}}->${{curr}}`);
                    }}
                    curr = pId;
                }}
            }}
        }});

        const selectedPathNodes = new Set();
        const selectedPathEdges = new Set();

        function termToString(term) {{
            if (!term) return '';
            if (typeof term === 'string') return term;
            if (typeof term === 'number') return String(term);
            if (term.Var !== undefined) return term.Var;
            if (term.Const !== undefined) return term.Const;
            if (term.Func !== undefined) {{
                const name = term.Func[0];
                const args = term.Func[1] || [];
                if (args.length === 1 && (name === '-' || name === '!')) {{
                    return `${{name}}${{termToString(args[0])}}`;
                }}
                const binaryOps = new Set(['+', '*', '·', '-', '/', '^', '&', '|', '<=', '<', '>=', '>', '=']);
                if (args.length === 2 && binaryOps.has(name)) {{
                    return `(${{termToString(args[0])}} ${{name}} ${{termToString(args[1])}})`;
                }}
                return `${{name}}(${{args.map(termToString).join(', ')}})`;
            }}
            return JSON.stringify(term);
        }}

        function equalityToString(eq) {{
            if (!eq) return '';
            if (typeof eq === 'string') return eq;
            if (eq.lhs !== undefined && eq.rhs !== undefined) {{
                return `${{termToString(eq.lhs)}} = ${{termToString(eq.rhs)}}`;
            }}
            return JSON.stringify(eq);
        }}

        function goalToString(g) {{
            if (!g) return '';
            if (g.equality) return equalityToString(g.equality);
            return equalityToString(g);
        }}

        function formatTacticShort(tactic) {{
            if (!tactic) return '';
            if (typeof tactic === 'string') {{
                if (tactic === 'Symmetry') return 'symm';
                if (tactic === 'Reflexivity') return 'rfl';
                if (tactic === 'EvalArithmeticLhs') return 'eval_lhs';
                if (tactic === 'EvalArithmeticRhs') return 'eval_rhs';
                if (tactic === 'TransposeToZero') return 'trans_zero';
                if (tactic === 'ZeroProductSplit') return 'split_zero';
                return tactic;
            }}
            if (tactic.RewriteLhs) return `rw [${{tactic.RewriteLhs}}]`;
            if (tactic.RewriteRhs) return `rw [${{tactic.RewriteRhs}}] (R)`;
            if (tactic.ApplyAxiom) return `apply ${{tactic.ApplyAxiom}}`;
            if (tactic.Transitivity) return `trans (${{termToString(tactic.Transitivity)}})`;
            return JSON.stringify(tactic);
        }}

        function computeTreeLayout() {{
            nodes.forEach(n => {{
                if (n.children_ids && n.children_ids.length > 0) {{
                    n.children_ids.sort((a, b) => {{
                        const rankA = primaryPathSet.has(a) ? 2 : (allProvenPathNodes.has(a) ? 1 : 0);
                        const rankB = primaryPathSet.has(b) ? 2 : (allProvenPathNodes.has(b) ? 1 : 0);
                        if (rankA !== rankB) return rankB - rankA;
                        const nodeA = nodeMap.get(a);
                        const nodeB = nodeMap.get(b);
                        const vA = nodeA ? nodeA.visit_count : 0;
                        const vB = nodeB ? nodeB.visit_count : 0;
                        return vB - vA;
                    }});
                }}
            }});

            const xSpacing = 220;
            const ySpacing = 36;
            let nextY = 0;

            function layoutSubtree(nodeId) {{
                const node = nodeMap.get(nodeId);
                if (!node) return;

                node.x = (node.depth || 0) * xSpacing + 80;

                const children = (node.children_ids || [])
                    .map(cid => nodeMap.get(cid))
                    .filter(Boolean);

                if (children.length === 0) {{
                    node.y = nextY;
                    nextY += ySpacing;
                }} else {{
                    children.forEach(c => layoutSubtree(c.id));
                    const firstChild = children[0];
                    const lastChild = children[children.length - 1];
                    node.y = (firstChild.y + lastChild.y) / 2;
                }}
            }}

            if (nodes.length > 0) {{
                layoutSubtree(0);
            }}

            nodes.forEach(node => {{
                if (node.x === undefined || node.y === undefined) {{
                    node.x = (node.depth || 0) * xSpacing + 80;
                    node.y = nextY;
                    nextY += ySpacing;
                }}
            }});
        }}
        computeTreeLayout();

        function render() {{
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            ctx.save();
            ctx.translate(panX, panY);
            ctx.scale(zoom, zoom);

            const viewLeft = -panX / zoom - 120;
            const viewRight = (canvas.width - panX) / zoom + 120;
            const viewTop = -panY / zoom - 120;
            const viewBottom = (canvas.height - panY) / zoom + 120;

            function isPointInView(x, y, margin = 60) {{
                return x >= viewLeft - margin && x <= viewRight + margin &&
                       y >= viewTop - margin && y <= viewBottom + margin;
            }}

            nodes.forEach(node => {{
                if (node.parent_id !== null && nodeMap.has(node.parent_id)) {{
                    const parent = nodeMap.get(node.parent_id);
                    const edgeKey = `${{parent.id}}->${{node.id}}`;
                    const isPrimaryEdge = primaryEdges.has(edgeKey);
                    const isProvenBranch = allProvenEdges.has(edgeKey);
                    const isSelectedEdge = selectedPathEdges.has(edgeKey);
                    const isHighlightedEdge = isPrimaryEdge || isProvenBranch || isSelectedEdge;

                    if (!isHighlightedEdge) {{
                        if ((parent.x < viewLeft && node.x < viewLeft) ||
                            (parent.x > viewRight && node.x > viewRight) ||
                            (parent.y < viewTop && node.y < viewTop) ||
                            (parent.y > viewBottom && node.y > viewBottom)) {{
                            return;
                        }}
                    }}

                    ctx.beginPath();
                    if (isSelectedEdge) {{
                        const isSelectedProven = (nodeMap.get(selectedNodeId) && nodeMap.get(selectedNodeId).is_proven) || allProvenPathNodes.has(selectedNodeId);
                        ctx.strokeStyle = isSelectedProven ? '#3fb950' : '#58a6ff';
                        ctx.lineWidth = 3.5;
                    }} else if (isPrimaryEdge) {{
                        ctx.strokeStyle = '#3fb950';
                        ctx.lineWidth = 3.2;
                    }} else if (isProvenBranch) {{
                        ctx.strokeStyle = '#238636';
                        ctx.lineWidth = 2.4;
                    }} else {{
                        ctx.strokeStyle = '#30363d';
                        ctx.lineWidth = 1.2;
                    }}

                    const midX = (parent.x + node.x) / 2;
                    ctx.moveTo(parent.x, parent.y);
                    ctx.bezierCurveTo(midX, parent.y, midX, node.y, node.x, node.y);
                    ctx.stroke();

                    if (node.applied_tactic && (isHighlightedEdge || isPointInView(midX, (parent.y + node.y) / 2))) {{
                        const tacticStr = formatTacticShort(node.applied_tactic);

                        const labelX = (parent.x + node.x) / 2;
                        const labelY = (parent.y + node.y) / 2 - 8;

                        ctx.font = '500 10px "SFMono-Regular", Consolas, monospace';
                        const textWidth = ctx.measureText(tacticStr).width;

                        let fillStyle, strokeStyle, textColor;
                        if (isSelectedEdge) {{
                            const isSelectedProven = (nodeMap.get(selectedNodeId) && nodeMap.get(selectedNodeId).is_proven) || allProvenPathNodes.has(selectedNodeId);
                            fillStyle = isSelectedProven ? '#112211' : '#041527';
                            strokeStyle = isSelectedProven ? '#2ea043' : '#1f6feb';
                            textColor = isSelectedProven ? '#3fb950' : '#58a6ff';
                        }} else if (isPrimaryEdge) {{
                            fillStyle = '#112211';
                            strokeStyle = '#2ea043';
                            textColor = '#3fb950';
                        }} else if (isProvenBranch) {{
                            fillStyle = '#0d1f12';
                            strokeStyle = '#1b4724';
                            textColor = '#2ea043';
                        }} else {{
                            fillStyle = '#161b22';
                            strokeStyle = '#30363d';
                            textColor = '#8b949e';
                        }}

                        ctx.fillStyle = fillStyle;
                        ctx.fillRect(labelX - textWidth / 2 - 4, labelY - 7, textWidth + 8, 14);
                        ctx.strokeStyle = strokeStyle;
                        ctx.lineWidth = 1;
                        ctx.strokeRect(labelX - textWidth / 2 - 4, labelY - 7, textWidth + 8, 14);

                        ctx.fillStyle = textColor;
                        ctx.textAlign = 'center';
                        ctx.textBaseline = 'middle';
                        ctx.fillText(tacticStr, labelX, labelY);
                    }}
                }}
            }});

            nodes.forEach(node => {{
                const radius = Math.min(22, Math.max(14, 14 + Math.log2(node.visit_count + 1) * 2.2));
                const isSelected = node.id === selectedNodeId;
                const isOnSelectedPath = selectedPathNodes.has(node.id);
                const isProvenTerminal = node.is_proven;
                const isPrimaryPath = primaryPathSet.has(node.id);
                const isProvenPath = allProvenPathNodes.has(node.id);

                if (!isProvenTerminal && !isProvenPath && !isSelected && !isOnSelectedPath && !isPointInView(node.x, node.y, radius + 10)) {{
                    return;
                }}

                ctx.beginPath();
                ctx.arc(node.x, node.y, radius, 0, 2 * Math.PI);

                if (isSelected) {{
                    ctx.fillStyle = isProvenTerminal ? '#2ea043' : (isProvenPath ? '#1b4724' : '#1f6feb');
                    ctx.strokeStyle = isProvenTerminal ? '#56d364' : (isProvenPath ? '#3fb950' : '#79c0ff');
                    ctx.lineWidth = 3.5;
                }} else if (isProvenTerminal) {{
                    ctx.fillStyle = '#238636';
                    ctx.strokeStyle = '#3fb950';
                    ctx.lineWidth = 3.0;
                }} else if (isPrimaryPath) {{
                    ctx.fillStyle = '#1b4724';
                    ctx.strokeStyle = '#3fb950';
                    ctx.lineWidth = 2.2;
                }} else if (isProvenPath) {{
                    ctx.fillStyle = '#122f19';
                    ctx.strokeStyle = '#2ea043';
                    ctx.lineWidth = 1.8;
                }} else if (isOnSelectedPath) {{
                    ctx.fillStyle = '#0d2d6b';
                    ctx.strokeStyle = '#58a6ff';
                    ctx.lineWidth = 2.0;
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

        function selectNode(nodeId) {{
            const node = nodeMap.get(nodeId);
            if (!node) return;
            selectedNodeId = nodeId;

            selectedPathNodes.clear();
            selectedPathEdges.clear();
            let curr = nodeId;
            const pathList = [];
            while (curr !== null && curr !== undefined && nodeMap.has(curr)) {{
                selectedPathNodes.add(curr);
                pathList.push(curr);
                const pId = nodeMap.get(curr).parent_id;
                if (pId !== null && pId !== undefined) {{
                    selectedPathEdges.add(`${{pId}}->${{curr}}`);
                }}
                curr = pId;
            }}
            pathList.reverse();

            updateInspector(node, pathList);
            updateTacticsSidebar(node);
            updateAstForNode(node);
            render();
            if (currentView === '3d') {{
                renderGalaxy();
            }}
        }}

        function updateInspector(node, pathList = []) {{
            if (!node) return;
            const container = document.getElementById('selectedNodeDetails');
            const state = node.state || {{}};
            const openGoals = state.open_goals || [];
            const goalsHtml = openGoals.length === 0
                ? '<span style="color:var(--accent-success); font-weight:600;">Reflexivity / All Goals Solved</span>'
                : openGoals.map(g => `<code style="color:#58a6ff; display:block; margin:2px 0; word-break:break-all;">${{goalToString(g)}}</code>`).join('');

            const inboundTactic = node.applied_tactic
                ? formatTacticShort(node.applied_tactic)
                : 'Root (Conjecture)';

            const isProvenNode = node.is_proven;
            const isProvenBranch = allProvenPathNodes.has(node.id);
            const statusColor = isProvenNode
                ? 'var(--accent-success)'
                : (isProvenBranch ? '#3fb950' : (node.is_terminal ? '#f85149' : 'var(--text-secondary)'));
            const statusText = isProvenNode
                ? 'Proven (Q.E.D.)'
                : (isProvenBranch ? 'On Proven Branch' : (node.is_terminal ? 'Dead End (Terminal)' : 'Active Open State'));

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
                        <span class="stat-value" style="color:${{statusColor}}; font-weight:600;">
                            ${{statusText}}
                        </span>
                    </div>
                </div>
                <div style="font-size:11px; color:var(--text-secondary); margin-bottom:4px; display:flex; justify-content:space-between;">
                    <span>Open Goals (${{openGoals.length}}):</span>
                    <span style="color:#8b949e;">Inbound: <code>${{inboundTactic}}</code></span>
                </div>
                <div style="background:#090d13; padding:10px; border-radius:6px; font-size:12px; font-family:var(--font-code); border:1px solid #30363d; max-height:160px; overflow-y:auto;">${{goalsHtml}}</div>
                ${{pathList.length > 1 ? `
                <div style="margin-top:10px; font-size:11px; color:var(--text-secondary);">
                    Path from Root: <span style="color:#c9d1d9;">${{pathList.join(' &rarr; ')}}</span>
                </div>` : ''}}
            `;
        }}

        function updateTacticsSidebar(node) {{
            const list = document.getElementById('tacticsList');
            if (!list) return;
            list.innerHTML = '';

            const history = (node && node.state && node.state.proof_history && node.state.proof_history.length > 0)
                ? node.state.proof_history.map(h => h[1])
                : (trace.tactics_sequence || []);

            if (history.length === 0) {{
                list.innerHTML = '<li style="color:var(--text-secondary); font-size:12px;">No proof steps found</li>';
                return;
            }}

            history.forEach((stepStr, idx) => {{
                const li = document.createElement('li');
                li.className = 'path-item';
                li.textContent = `${{idx + 1}}. ${{stepStr}}`;
                list.appendChild(li);
            }});
        }}

        function focusProvenPath() {{
            let targetNodeId = (selectedNodeId !== null && nodeMap.has(selectedNodeId) && nodeMap.get(selectedNodeId).is_proven)
                ? selectedNodeId
                : (trace.proven_node_id !== null ? trace.proven_node_id : 0);

            const pathNodeIds = [];
            let curr = targetNodeId;
            while (curr !== null && curr !== undefined && nodeMap.has(curr)) {{
                pathNodeIds.push(curr);
                curr = nodeMap.get(curr).parent_id;
            }}

            const pathNodes = pathNodeIds.map(id => nodeMap.get(id)).filter(Boolean);
            if (pathNodes.length === 0) return;

            let minX = Infinity, maxX = -Infinity;
            let minY = Infinity, maxY = -Infinity;

            pathNodes.forEach(n => {{
                if (n.x < minX) minX = n.x;
                if (n.x > maxX) maxX = n.x;
                if (n.y < minY) minY = n.y;
                if (n.y > maxY) maxY = n.y;
            }});

            const pathWidth = maxX - minX + 160;
            const pathHeight = maxY - minY + 160;

            const scaleX = (canvas.width - 100) / pathWidth;
            const scaleY = (canvas.height - 100) / pathHeight;
            const targetZoom = Math.max(0.35, Math.min(1.2, Math.min(scaleX, scaleY)));

            const centerX = (minX + maxX) / 2;
            const centerY = (minY + maxY) / 2;

            zoom = targetZoom;
            panX = canvas.width / 2 - centerX * zoom;
            panY = canvas.height / 2 - centerY * zoom;

            selectNode(targetNodeId);
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
                selectNode(closest.id);
            }}
        }});

        document.getElementById('btnZoomIn').onclick = () => {{
            if (currentView === '3d') {{
                galaxyCamDist = Math.max(80, galaxyCamDist - 40);
                renderGalaxy();
            }} else {{
                zoom = Math.min(3.0, zoom * 1.25);
                render();
            }}
        }};
        document.getElementById('btnZoomOut').onclick = () => {{
            if (currentView === '3d') {{
                galaxyCamDist = Math.min(800, galaxyCamDist + 40);
                renderGalaxy();
            }} else {{
                zoom = Math.max(0.15, zoom * 0.8);
                render();
            }}
        }};
        document.getElementById('btnResetView').onclick = () => {{
            if (currentView === '3d') {{
                galaxyRotX = 0.35;
                galaxyRotY = -0.55;
                galaxyCamDist = 340;
                galaxyPanX = 0;
                galaxyPanY = 0;
                renderGalaxy();
            }} else {{
                panX = 100;
                panY = 150;
                zoom = 1.0;
                render();
            }}
        }};
        document.getElementById('btnCenterProven').onclick = () => {{
            focusProvenPath();
            if (currentView === '3d') {{
                galaxyRotX = 0.35;
                galaxyRotY = -0.55;
                galaxyCamDist = 340;
                galaxyPanX = 0;
                galaxyPanY = 0;
                renderGalaxy();
            }}
        }};

        const DOMAIN_PALETTE = {{
            'Boolean Logic': {{ base: '#b060ff', glow: 'rgba(176, 96, 255, 0.45)', fill: '#e9d5ff' }},
            'Symbolic Calculus': {{ base: '#ff9800', glow: 'rgba(255, 152, 0, 0.45)', fill: '#fed7aa' }},
            'Set Theory & Induction': {{ base: '#26a69a', glow: 'rgba(38, 166, 154, 0.45)', fill: '#99f6e4' }},
            'Linear Algebra': {{ base: '#00bcd4', glow: 'rgba(0, 188, 212, 0.45)', fill: '#a5f3fc' }},
            'Abstract Algebra': {{ base: '#2979ff', glow: 'rgba(41, 121, 255, 0.45)', fill: '#bfdbfe' }}
        }};

        function getDomainForTrace() {{
            const d = trace.domain || 'Abstract Algebra';
            if (d.indexOf('Bool') !== -1) return 'Boolean Logic';
            if (d.indexOf('Calc') !== -1 || d.indexOf('ODE') !== -1) return 'Symbolic Calculus';
            if (d.indexOf('Set') !== -1 || d.indexOf('Induct') !== -1) return 'Set Theory & Induction';
            if (d.indexOf('Lin') !== -1 || d.indexOf('Matrix') !== -1) return 'Linear Algebra';
            return 'Abstract Algebra';
        }}

        function initVectorSpace3D() {{
            if (!nodes || nodes.length === 0) return;
            const N = nodes.length;
            const D = 8;
            const vectors = [];

            nodes.forEach(node => {{
                const d = node.depth || 0;
                const mv = node.mean_value || 0;
                const pp = node.policy_prior || 0;
                const vc = Math.log2((node.visit_count || 0) + 1);
                const pr = node.is_proven ? 1.0 : 0.0;
                const og = (node.state && node.state.open_goals) ? node.state.open_goals.length : 0;
                let tacVal = 0.0;
                if (node.applied_tactic) {{
                    if (node.applied_tactic.RewriteLhs) tacVal = 1.0;
                    else if (node.applied_tactic.RewriteRhs) tacVal = 2.0;
                    else tacVal = 0.5;
                }}
                const hash = ((node.id * 17) % 31) / 31.0;
                vectors.push([d, mv, pp, vc, pr, og, tacVal, hash]);
            }});

            const mean = new Array(D).fill(0);
            for (let i = 0; i < N; i++) {{
                for (let j = 0; j < D; j++) {{
                    mean[j] += vectors[i][j];
                }}
            }}
            for (let j = 0; j < D; j++) {{
                mean[j] /= N;
            }}

            const centered = [];
            for (let i = 0; i < N; i++) {{
                const c = [];
                for (let j = 0; j < D; j++) {{
                    c.push(vectors[i][j] - mean[j]);
                }}
                centered.push(c);
            }}

            function dot(v1, v2) {{
                let s = 0;
                for (let j = 0; j < D; j++) s += v1[j] * v2[j];
                return s;
            }}

            const basis = [];
            for (let k = 0; k < 3; k++) {{
                let w = [];
                for (let j = 0; j < D; j++) {{
                    w.push(((j + 1) * (k + 1) * 31 % 97) - 48);
                }}
                for (let b = 0; b < basis.length; b++) {{
                    const pr = dot(w, basis[b]);
                    for (let j = 0; j < D; j++) w[j] -= pr * basis[b][j];
                }}
                let norm = Math.hypot.apply(Math, w);
                if (norm > 1e-9) {{
                    for (let j = 0; j < D; j++) w[j] /= norm;
                }}

                for (let it = 0; it < 20; it++) {{
                    const nextW = new Array(D).fill(0);
                    for (let i = 0; i < N; i++) {{
                        const d = dot(centered[i], w);
                        for (let j = 0; j < D; j++) nextW[j] += centered[i][j] * d;
                    }}
                    for (let j = 0; j < D; j++) nextW[j] /= N;
                    for (let b = 0; b < basis.length; b++) {{
                        const pr = dot(nextW, basis[b]);
                        for (let j = 0; j < D; j++) nextW[j] -= pr * basis[b][j];
                    }}
                    const nNorm = Math.hypot.apply(Math, nextW);
                    if (nNorm > 1e-9) {{
                        for (let j = 0; j < D; j++) w[j] = nextW[j] / nNorm;
                    }} else {{
                        break;
                    }}
                }}
                basis.push(w);
            }}

            let maxExtent = 0;
            const rawPoints = [];
            for (let i = 0; i < N; i++) {{
                const c = centered[i];
                const x = basis.length > 0 ? dot(c, basis[0]) : 0;
                const y = basis.length > 1 ? dot(c, basis[1]) : 0;
                const z = basis.length > 2 ? dot(c, basis[2]) : 0;
                maxExtent = Math.max(maxExtent, Math.abs(x), Math.abs(y), Math.abs(z));
                rawPoints.push({{ x: x, y: y, z: z, node: nodes[i] }});
            }}

            const scale = maxExtent > 1e-6 ? (120.0 / maxExtent) : 1.0;
            const domainName = getDomainForTrace();

            vectorSpace3D.points = rawPoints.map((rp, i) => {{
                let px = rp.x * scale;
                let py = rp.y * scale;
                let pz = rp.z * scale;
                if (maxExtent <= 1e-6 && N > 1) {{
                    const ang = (i * 2.0 * Math.PI) / N;
                    px = 40.0 * Math.cos(ang);
                    py = ((i / N) - 0.5) * 60.0;
                    pz = 40.0 * Math.sin(ang);
                }}
                const node = rp.node;
                const stmt = (node.state && node.state.open_goals && node.state.open_goals.length > 0)
                    ? goalToString(node.state.open_goals[0])
                    : (node.is_proven ? 'Q.E.D.' : trace.conjecture);

                return {{
                    id: node.id,
                    name: 'Node #' + node.id + (node.is_proven ? ' (Q.E.D.)' : (node.id === 0 ? ' (Root)' : '')),
                    statement: stmt,
                    domain: domainName,
                    tactic_name: formatTacticShort(node.applied_tactic),
                    proof_length: node.depth || 1,
                    is_proven: !!node.is_proven,
                    x: Math.round(px * 100) / 100,
                    y: Math.round(py * 100) / 100,
                    z: Math.round(pz * 100) / 100
                }};
            }});

            const ptsMap = new Map();
            vectorSpace3D.points.forEach(p => ptsMap.set(p.id, p));

            const traj = [];
            (trace.proof_path_node_ids || []).forEach((nodeId, stepIdx) => {{
                const p = ptsMap.get(nodeId);
                if (p) {{
                    traj.push({{
                        step: stepIdx,
                        node_id: nodeId,
                        state_str: p.statement,
                        x: p.x,
                        y: p.y,
                        z: p.z
                    }});
                }}
            }});
            vectorSpace3D.trajectory = traj;
            vectorSpace3D.total_nodes = vectorSpace3D.points.length;

            const nEl = document.getElementById('galaxyNodesCount');
            const tEl = document.getElementById('galaxyTrajCount');
            if (nEl) nEl.textContent = String(vectorSpace3D.total_nodes);
            if (tEl) tEl.textContent = String(vectorSpace3D.trajectory.length);
        }}

        function renderGalaxy() {{
            if (!galaxyCanvas || !galaxyCtx) return;
            const w = galaxyCanvas.width;
            const h = galaxyCanvas.height;
            if (w <= 0 || h <= 0) return;

            galaxyCtx.clearRect(0, 0, w, h);

            const cx = w / 2;
            const cy = h / 2;
            const bgGrad = galaxyCtx.createRadialGradient(cx, cy, 40, cx, cy, Math.max(cx, cy));
            bgGrad.addColorStop(0, '#131b2e');
            bgGrad.addColorStop(0.65, '#0b0f19');
            bgGrad.addColorStop(1, '#05070c');
            galaxyCtx.fillStyle = bgGrad;
            galaxyCtx.fillRect(0, 0, w, h);

            galaxyCtx.fillStyle = 'rgba(255, 255, 255, 0.35)';
            for (let i = 0; i < 75; i++) {{
                const sx = (Math.sin(i * 19.3) * 0.5 + 0.5) * w;
                const sy = (Math.cos(i * 27.7) * 0.5 + 0.5) * h;
                const sr = (i % 3 === 0) ? 1.4 : 0.9;
                galaxyCtx.beginPath();
                galaxyCtx.arc(sx, sy, sr, 0, Math.PI * 2);
                galaxyCtx.fill();
            }}

            const cosY = Math.cos(galaxyRotY);
            const sinY = Math.sin(galaxyRotY);
            const cosX = Math.cos(galaxyRotX);
            const sinX = Math.sin(galaxyRotX);
            const fov = 420;

            function project(x, y, z) {{
                const x1 = x * cosY + z * sinY;
                const z1 = -x * sinY + z * cosY;
                const y2 = y * cosX - z1 * sinX;
                const z2 = y * sinX + z1 * cosX;
                const zCam = z2 + galaxyCamDist;
                if (zCam <= 10) return null;
                const scale = fov / zCam;
                const sx = cx + galaxyPanX + x1 * scale;
                const sy = cy + galaxyPanY - y2 * scale;
                return {{ sx: sx, sy: sy, scale: scale, zCam: zCam }};
            }}

            galaxyCtx.lineWidth = 1;
            galaxyCtx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
            const gridRange = 140;
            const gridStep = 35;
            for (let g = -gridRange; g <= gridRange; g += gridStep) {{
                const p1 = project(g, 0, -gridRange);
                const p2 = project(g, 0, gridRange);
                if (p1 && p2) {{
                    galaxyCtx.beginPath();
                    galaxyCtx.moveTo(p1.sx, p1.sy);
                    galaxyCtx.lineTo(p2.sx, p2.sy);
                    galaxyCtx.stroke();
                }}
                const p3 = project(-gridRange, 0, g);
                const p4 = project(gridRange, 0, g);
                if (p3 && p4) {{
                    galaxyCtx.beginPath();
                    galaxyCtx.moveTo(p3.sx, p3.sy);
                    galaxyCtx.lineTo(p4.sx, p4.sy);
                    galaxyCtx.stroke();
                }}
            }}

            const traj = vectorSpace3D.trajectory || [];
            if (traj.length > 1) {{
                galaxyCtx.strokeStyle = 'rgba(52, 211, 153, 0.85)';
                galaxyCtx.lineWidth = 2.5;
                galaxyCtx.setLineDash([5, 4]);
                galaxyCtx.beginPath();
                let started = false;
                for (let i = 0; i < traj.length; i++) {{
                    const pr = project(traj[i].x, traj[i].y, traj[i].z);
                    if (pr) {{
                        if (!started) {{
                            galaxyCtx.moveTo(pr.sx, pr.sy);
                            started = true;
                        }} else {{
                            galaxyCtx.lineTo(pr.sx, pr.sy);
                        }}
                    }}
                }}
                galaxyCtx.stroke();
                galaxyCtx.setLineDash([]);
            }}

            galaxyProjectedItems = [];

            const pts = vectorSpace3D.points || [];
            pts.forEach(pt => {{
                const pr = project(pt.x, pt.y, pt.z);
                if (pr) {{
                    const radius = Math.max(3.5, Math.min(22, (4.8 + (pt.proof_length || 1) * 0.7) * pr.scale));
                    galaxyProjectedItems.push({{
                        type: 'node',
                        pt: pt,
                        sx: pr.sx,
                        sy: pr.sy,
                        scale: pr.scale,
                        radius: radius,
                        zCam: pr.zCam
                    }});
                }}
            }});

            traj.forEach(tr => {{
                const pr = project(tr.x, tr.y, tr.z);
                if (pr) {{
                    galaxyProjectedItems.push({{
                        type: 'traj',
                        tr: tr,
                        sx: pr.sx,
                        sy: pr.sy,
                        scale: pr.scale,
                        radius: 8 * pr.scale,
                        zCam: pr.zCam
                    }});
                }}
            }});

            galaxyProjectedItems.sort((a, b) => b.zCam - a.zCam);

            galaxyProjectedItems.forEach(item => {{
                if (item.type === 'node') {{
                    const pt = item.pt;
                    const isSelected = (pt.id === selectedNodeId);
                    const isHovered = (hoveredPoint3D && hoveredPoint3D.id === pt.id);
                    const colors = DOMAIN_PALETTE[pt.domain] || DOMAIN_PALETTE['Abstract Algebra'];
                    const r = isSelected ? item.radius * 1.35 : (isHovered ? item.radius * 1.2 : item.radius);

                    const glowGrad = galaxyCtx.createRadialGradient(item.sx, item.sy, r * 0.2, item.sx, item.sy, r * 2.4);
                    glowGrad.addColorStop(0, isSelected ? 'rgba(251, 191, 36, 0.6)' : colors.glow);
                    glowGrad.addColorStop(1, 'rgba(0, 0, 0, 0)');
                    galaxyCtx.fillStyle = glowGrad;
                    galaxyCtx.beginPath();
                    galaxyCtx.arc(item.sx, item.sy, r * 2.4, 0, Math.PI * 2);
                    galaxyCtx.fill();

                    const sphereGrad = galaxyCtx.createRadialGradient(item.sx - r * 0.3, item.sy - r * 0.3, r * 0.1, item.sx, item.sy, r);
                    sphereGrad.addColorStop(0, '#ffffff');
                    sphereGrad.addColorStop(0.3, colors.fill);
                    sphereGrad.addColorStop(0.8, colors.base);
                    sphereGrad.addColorStop(1, '#0f172a');
                    galaxyCtx.fillStyle = sphereGrad;
                    galaxyCtx.beginPath();
                    galaxyCtx.arc(item.sx, item.sy, r, 0, Math.PI * 2);
                    galaxyCtx.fill();

                    galaxyCtx.strokeStyle = isSelected ? '#fbbf24' : (isHovered ? '#ffffff' : colors.base);
                    galaxyCtx.lineWidth = isSelected ? 2.5 : (isHovered ? 2.0 : 1.0);
                    galaxyCtx.stroke();

                    if (isSelected) {{
                        galaxyCtx.strokeStyle = '#fbbf24';
                        galaxyCtx.lineWidth = 1.5;
                        galaxyCtx.setLineDash([3, 3]);
                        galaxyCtx.beginPath();
                        galaxyCtx.arc(item.sx, item.sy, r * 2.2, 0, Math.PI * 2);
                        galaxyCtx.stroke();
                        galaxyCtx.setLineDash([]);
                    }}

                    if (isHovered || isSelected || item.scale > 1.25) {{
                        galaxyCtx.font = '600 10px "SFMono-Regular", Consolas, monospace';
                        const nameText = pt.name;
                        const tw = galaxyCtx.measureText(nameText).width;
                        const bx = item.sx + r + 6;
                        const by = item.sy - 8;

                        galaxyCtx.fillStyle = 'rgba(15, 23, 42, 0.85)';
                        galaxyCtx.fillRect(bx - 3, by - 2, tw + 6, 16);
                        galaxyCtx.strokeStyle = colors.base;
                        galaxyCtx.lineWidth = 1;
                        galaxyCtx.strokeRect(bx - 3, by - 2, tw + 6, 16);

                        galaxyCtx.fillStyle = '#ffffff';
                        galaxyCtx.textBaseline = 'middle';
                        galaxyCtx.textAlign = 'left';
                        galaxyCtx.fillText(nameText, bx, by + 6);
                    }}
                }} else if (item.type === 'traj') {{
                    const tr = item.tr;
                    galaxyCtx.fillStyle = '#10b981';
                    galaxyCtx.beginPath();
                    galaxyCtx.arc(item.sx, item.sy, Math.max(5, item.radius), 0, Math.PI * 2);
                    galaxyCtx.fill();
                    galaxyCtx.strokeStyle = '#34d399';
                    galaxyCtx.lineWidth = 2;
                    galaxyCtx.stroke();

                    galaxyCtx.fillStyle = '#ffffff';
                    galaxyCtx.font = '700 8px "SFMono-Regular", Consolas, monospace';
                    galaxyCtx.textAlign = 'center';
                    galaxyCtx.textBaseline = 'middle';
                    galaxyCtx.fillText(String(tr.step + 1), item.sx, item.sy);
                }}
            }});
        }}

        galaxyCanvas.addEventListener('mousedown', e => {{
            isGalaxyDragging = true;
            isGalaxyPanDrag = (e.button === 2 || e.shiftKey);
            galaxyDragStartX = e.clientX;
            galaxyDragStartY = e.clientY;
            galaxyCanvas.style.cursor = 'grabbing';
        }});

        window.addEventListener('mouseup', () => {{
            if (isGalaxyDragging) {{
                isGalaxyDragging = false;
                if (galaxyCanvas) galaxyCanvas.style.cursor = 'grab';
            }}
        }});

        galaxyCanvas.addEventListener('contextmenu', e => e.preventDefault());

        galaxyCanvas.addEventListener('mousemove', e => {{
            const rect = galaxyCanvas.getBoundingClientRect();
            const mouseX = e.clientX - rect.left;
            const mouseY = e.clientY - rect.top;

            if (isGalaxyDragging) {{
                const dx = e.clientX - galaxyDragStartX;
                const dy = e.clientY - galaxyDragStartY;
                galaxyDragStartX = e.clientX;
                galaxyDragStartY = e.clientY;

                if (isGalaxyPanDrag) {{
                    galaxyPanX += dx;
                    galaxyPanY += dy;
                }} else {{
                    galaxyRotY += dx * 0.008;
                    galaxyRotX = Math.max(-1.45, Math.min(1.45, galaxyRotX - dy * 0.008));
                    const yawDeg = Math.round((galaxyRotY * 180 / Math.PI)) % 360;
                    const pitchDeg = Math.round(galaxyRotX * 180 / Math.PI);
                    const yawEl = document.getElementById('galaxyYawVal');
                    const pitchEl = document.getElementById('galaxyPitchVal');
                    if (yawEl) yawEl.textContent = yawDeg + '°';
                    if (pitchEl) pitchEl.textContent = pitchDeg + '°';
                }}
                renderGalaxy();
            }} else {{
                let found = null;
                for (let i = galaxyProjectedItems.length - 1; i >= 0; i--) {{
                    const item = galaxyProjectedItems[i];
                    if (item.type === 'node') {{
                        const dist = Math.hypot(item.sx - mouseX, item.sy - mouseY);
                        if (dist <= Math.max(14, item.radius + 6)) {{
                            found = item.pt;
                            break;
                        }}
                    }}
                }}
                if (found !== hoveredPoint3D) {{
                    hoveredPoint3D = found;
                    const tt = document.getElementById('galaxyTooltip');
                    if (hoveredPoint3D && tt) {{
                        document.getElementById('galaxyTtTitle').textContent = hoveredPoint3D.name;
                        document.getElementById('galaxyTtDomain').textContent = hoveredPoint3D.domain + ' | Steps: ' + hoveredPoint3D.proof_length;
                        document.getElementById('galaxyTtStmt').textContent = hoveredPoint3D.statement;
                        document.getElementById('galaxyTtTactic').textContent = hoveredPoint3D.tactic_name || '';
                        tt.style.display = 'block';
                        const maxX = galaxyCanvas.width - 320;
                        const maxY = galaxyCanvas.height - 120;
                        tt.style.left = Math.min(maxX, mouseX + 16) + 'px';
                        tt.style.top = Math.min(maxY, mouseY + 10) + 'px';
                    }} else if (tt) {{
                        tt.style.display = 'none';
                    }}
                    renderGalaxy();
                }}
            }}
        }});

        galaxyCanvas.addEventListener('mouseleave', () => {{
            hoveredPoint3D = null;
            const tt = document.getElementById('galaxyTooltip');
            if (tt) tt.style.display = 'none';
            renderGalaxy();
        }});

        galaxyCanvas.addEventListener('wheel', e => {{
            e.preventDefault();
            const delta = e.deltaY < 0 ? -25 : 25;
            galaxyCamDist = Math.max(80, Math.min(800, galaxyCamDist + delta));
            renderGalaxy();
        }});

        galaxyCanvas.addEventListener('click', e => {{
            const rect = galaxyCanvas.getBoundingClientRect();
            const mouseX = e.clientX - rect.left;
            const mouseY = e.clientY - rect.top;

            let clicked = null;
            for (let i = galaxyProjectedItems.length - 1; i >= 0; i--) {{
                const item = galaxyProjectedItems[i];
                if (item.type === 'node') {{
                    const dist = Math.hypot(item.sx - mouseX, item.sy - mouseY);
                    if (dist <= Math.max(14, item.radius + 6)) {{
                        clicked = item.pt;
                        break;
                    }}
                }}
            }}
            if (clicked) {{
                selectNode(clicked.id);
            }}
        }});

        function updateAstForNode(node) {{
            if (!node) return;
            let eqStr = "";
            if (node.state && node.state.open_goals && node.state.open_goals.length > 0) {{
                const g = node.state.open_goals[0];
                eqStr = goalToString(g);
            }} else if (node.is_proven) {{
                eqStr = "Q.E.D.";
            }} else {{
                eqStr = trace.conjecture;
            }}

            let rewriteInfo = null;
            if (node.applied_tactic) {{
                const tac = node.applied_tactic;
                if (tac.RewriteLhs || (typeof tac === 'string' && tac.toLowerCase().indexOf('lhs') !== -1)) {{
                    rewriteInfo = {{ side: 'lhs', name: tac.RewriteLhs || tac }};
                }} else if (tac.RewriteRhs || (typeof tac === 'string' && tac.toLowerCase().indexOf('rhs') !== -1)) {{
                    rewriteInfo = {{ side: 'rhs', name: tac.RewriteRhs || tac }};
                }} else {{
                    rewriteInfo = {{ side: 'all', name: JSON.stringify(tac) }};
                }}
            }}
            renderEquationAst(eqStr, 'Node #' + node.id + (node.is_proven ? ' (Q.E.D.)' : ''), rewriteInfo);
        }}

        function renderEquationAst(rawEquation, title, rewriteInfo) {{
            const titleEl = document.getElementById('astTargetName');
            const exprEl = document.getElementById('astEquationExpr');
            const container = document.getElementById('astSvgContainer');
            const countEl = document.getElementById('astNodeCount');
            if (!container) return;

            if (titleEl) titleEl.textContent = title || 'Goal';
            if (exprEl) exprEl.textContent = rawEquation || '(empty)';

            const ast = parseEquationAST(rawEquation);

            if (rewriteInfo && ast) {{
                if (rewriteInfo.is_theorem) {{
                    if (ast.children && ast.children.length > 1) {{
                        markSubtreeRewritten(ast.children[1]);
                    }}
                }} else if (rewriteInfo.side === 'lhs' && ast.children && ast.children.length > 0) {{
                    markSubtreeRewritten(ast.children[0]);
                }} else if (rewriteInfo.side === 'rhs' && ast.children && ast.children.length > 1) {{
                    markSubtreeRewritten(ast.children[1]);
                }} else if (rewriteInfo.side === 'all') {{
                    markSubtreeRewritten(ast);
                }}
            }}

            function markSubtreeRewritten(node) {{
                if (!node) return;
                node.is_rewritten = true;
                if (node.children) {{
                    node.children.forEach(markSubtreeRewritten);
                }}
            }}

            function computeAstWidths(node) {{
                if (!node) return 0;
                if (!node.children || node.children.length === 0) {{
                    const charLen = (node.label || '').length;
                    node.width = Math.max(36, charLen * 7.5 + 14);
                    return node.width;
                }}
                let sumWidth = 0;
                node.children.forEach((child, idx) => {{
                    sumWidth += computeAstWidths(child);
                    if (idx > 0) sumWidth += 14;
                }});
                node.width = Math.max(36, sumWidth);
                return node.width;
            }}

            let maxAstY = 0;
            let totalAstNodes = 0;
            function assignAstPositions(node, xStart, y) {{
                if (!node) return;
                totalAstNodes++;
                node.y = y;
                if (y > maxAstY) maxAstY = y;
                if (!node.children || node.children.length === 0) {{
                    node.x = xStart + node.width / 2;
                    return;
                }}
                let currX = xStart;
                node.children.forEach(child => {{
                    assignAstPositions(child, currX, y + 36);
                    currX += child.width + 14;
                }});
                const firstX = node.children[0].x;
                const lastX = node.children[node.children.length - 1].x;
                node.x = (firstX + lastX) / 2;
            }}

            computeAstWidths(ast);
            const paddingX = 24;
            const contWidth = Math.max(380, container.clientWidth || 380);
            const startX = Math.max(paddingX, (contWidth - ast.width) / 2);
            assignAstPositions(ast, startX, 22);

            const svgWidth = Math.max(contWidth, ast.width + paddingX * 2);
            const svgHeight = Math.max(140, maxAstY + 36);

            let pathsSvg = '';
            let nodesSvg = '';

            function collectSvgElements(node, parent) {{
                if (!node) return;
                if (parent) {{
                    const isRewrittenEdge = node.is_rewritten || parent.is_rewritten;
                    const stroke = isRewrittenEdge ? '#e37400' : '#30363d';
                    const strokeWidth = isRewrittenEdge ? '2.5' : '1.5';
                    const midY = (parent.y + node.y) / 2;
                    pathsSvg += '<path d="M ' + parent.x + ' ' + (parent.y + 11) + ' C ' + parent.x + ' ' + midY + ', ' + node.x + ' ' + midY + ', ' + node.x + ' ' + (node.y - 10) + '" fill="none" stroke="' + stroke + '" stroke-width="' + strokeWidth + '" />';
                }}
                if (node.type === 'op') {{
                    const isEq = node.isEqualityRoot;
                    const r = isEq ? 13 : 11;
                    const fill = node.is_rewritten ? '#2d1a04' : (isEq ? '#0d2d6b' : '#161b22');
                    const stroke = node.is_rewritten ? '#e37400' : (isEq ? '#58a6ff' : '#8b949e');
                    const textFill = node.is_rewritten ? '#ffb74d' : (isEq ? '#58a6ff' : '#f0f6fc');
                    if (node.is_rewritten) {{
                        nodesSvg += '<circle cx="' + node.x + '" cy="' + node.y + '" r="' + (r + 4) + '" fill="none" stroke="#e37400" stroke-width="1.5" opacity="0.6" />';
                    }}
                    nodesSvg += '<circle cx="' + node.x + '" cy="' + node.y + '" r="' + r + '" fill="' + fill + '" stroke="' + stroke + '" stroke-width="2" />';
                    nodesSvg += '<text x="' + node.x + '" y="' + (node.y + 3.5) + '" text-anchor="middle" font-family="sans-serif" font-size="10px" font-weight="700" fill="' + textFill + '">' + escapeHtml(node.label) + '</text>';
                }} else {{
                    const charLen = (node.label || '').length;
                    const boxW = Math.max(26, charLen * 7.5 + 10);
                    const boxH = 18;
                    const rx = 5;
                    const fill = node.is_rewritten ? '#2d1a04' : '#090d13';
                    const stroke = node.is_rewritten ? '#e37400' : '#30363d';
                    const textFill = node.is_rewritten ? '#ffb74d' : '#c9d1d9';
                    if (node.is_rewritten) {{
                        nodesSvg += '<rect x="' + (node.x - boxW / 2 - 3) + '" y="' + (node.y - boxH / 2 - 3) + '" width="' + (boxW + 6) + '" height="' + (boxH + 6) + '" rx="' + (rx + 2) + '" fill="none" stroke="#e37400" stroke-width="1.5" opacity="0.6" />';
                    }}
                    nodesSvg += '<rect x="' + (node.x - boxW / 2) + '" y="' + (node.y - boxH / 2) + '" width="' + boxW + '" height="' + boxH + '" rx="' + rx + '" fill="' + fill + '" stroke="' + stroke + '" stroke-width="1.5" />';
                    nodesSvg += '<text x="' + node.x + '" y="' + (node.y + 3.5) + '" text-anchor="middle" font-family="monospace" font-size="9px" font-weight="500" fill="' + textFill + '">' + escapeHtml(node.label) + '</text>';
                }}

                if (node.children) {{
                    node.children.forEach(child => collectSvgElements(child, node));
                }}
            }}

            collectSvgElements(ast, null);

            container.innerHTML = '<svg width="' + svgWidth + '" height="' + svgHeight + '" xmlns="http://www.w3.org/2000/svg" style="display:block;">' + pathsSvg + nodesSvg + '</svg>';
            if (countEl) countEl.textContent = totalAstNodes + ' nodes';
        }}

        function parseEquationAST(rawStr) {{
            if (!rawStr || typeof rawStr !== 'string') {{
                return {{ id: 1, label: 'empty', type: 'leaf', children: [] }};
            }}
            const clean = rawStr.trim();
            if (clean === "Q.E.D.") {{
                return {{ id: 1, label: "Q.E.D.", type: "leaf", children: [] }};
            }}

            let eqIdx = -1;
            let parenDepth = 0;
            for (let i = 0; i < clean.length; i++) {{
                const c = clean[i];
                if (c === '(') parenDepth++;
                else if (c === ')') parenDepth--;
                else if (c === '=' && parenDepth === 0) {{
                    eqIdx = i;
                    break;
                }}
            }}

            let nextNodeId = 1;

            function tokenize(s) {{
                const tokens = [];
                let i = 0;
                while (i < s.length) {{
                    const ch = s[i];
                    if (/\s/.test(ch)) {{
                        i++;
                        continue;
                    }}
                    if (ch === '(' || ch === ')') {{
                        tokens.push({{ type: 'paren', value: ch }});
                        i++;
                    }} else if (['+', '*', '/', '^', '&', '|', '!'].indexOf(ch) !== -1) {{
                        tokens.push({{ type: 'op', value: ch }});
                        i++;
                    }} else if (ch === '-') {{
                        tokens.push({{ type: 'op', value: '-' }});
                        i++;
                    }} else {{
                        let j = i;
                        while (j < s.length && !/\s/.test(s[j]) && ['(', ')', '+', '*', '/', '^', '&', '|', '!', '-', '='].indexOf(s[j]) === -1) {{
                            j++;
                        }}
                        const val = s.slice(i, j).trim();
                        if (val.length > 0) {{
                            tokens.push({{ type: 'ident', value: val }});
                        }}
                        i = j;
                    }}
                }}
                return tokens;
            }}

            function parseTokens(tokens) {{
                if (!tokens || tokens.length === 0) return null;

                while (tokens.length >= 2 && tokens[0].value === '(' && tokens[tokens.length - 1].value === ')') {{
                    let depth = 0;
                    let wrapsAll = true;
                    for (let i = 0; i < tokens.length - 1; i++) {{
                        if (tokens[i].value === '(') depth++;
                        else if (tokens[i].value === ')') depth--;
                        if (depth === 0) {{
                            wrapsAll = false;
                            break;
                        }}
                    }}
                    if (wrapsAll) {{
                        tokens = tokens.slice(1, tokens.length - 1);
                    }} else {{
                        break;
                    }}
                }}

                if (tokens.length === 0) return null;
                if (tokens.length === 1) {{
                    return {{
                        id: nextNodeId++,
                        label: tokens[0].value,
                        type: 'leaf',
                        children: []
                    }};
                }}

                const opPrecedence = {{ '|': 1, '&': 2, '+': 3, '-': 3, '*': 4, '/': 4, '^': 5 }};
                let minPrec = 999;
                let splitIdx = -1;
                let depth = 0;

                for (let i = tokens.length - 1; i >= 0; i--) {{
                    const tok = tokens[i];
                    if (tok.value === ')') depth++;
                    else if (tok.value === '(') depth--;
                    else if (depth === 0 && tok.type === 'op' && opPrecedence[tok.value] !== undefined) {{
                        if (tok.value === '-' && (i === 0 || tokens[i - 1].type === 'op' || tokens[i - 1].value === '(')) {{
                            continue;
                        }}
                        const prec = opPrecedence[tok.value];
                        if (prec < minPrec) {{
                            minPrec = prec;
                            splitIdx = i;
                        }}
                    }}
                }}

                if (splitIdx !== -1) {{
                    const opTok = tokens[splitIdx];
                    const leftSub = parseTokens(tokens.slice(0, splitIdx));
                    const rightSub = parseTokens(tokens.slice(splitIdx + 1));
                    const children = [];
                    if (leftSub) children.push(leftSub);
                    if (rightSub) children.push(rightSub);
                    return {{
                        id: nextNodeId++,
                        label: opTok.value,
                        type: 'op',
                        children: children
                    }};
                }}

                if (tokens[0].type === 'op' && ['!', '-'].indexOf(tokens[0].value) !== -1) {{
                    const childSub = parseTokens(tokens.slice(1));
                    return {{
                        id: nextNodeId++,
                        label: tokens[0].value,
                        type: 'op',
                        children: childSub ? [childSub] : []
                    }};
                }}

                const label = tokens.map(t => t.value).join(' ');
                return {{
                    id: nextNodeId++,
                    label: label,
                    type: 'leaf',
                    children: []
                }};
            }}

            if (eqIdx !== -1) {{
                const leftStr = clean.slice(0, eqIdx).trim();
                const rightStr = clean.slice(eqIdx + 1).trim();
                const leftAst = parseTokens(tokenize(leftStr));
                const rightAst = parseTokens(tokenize(rightStr));
                const children = [];
                if (leftAst) children.push(leftAst);
                if (rightAst) children.push(rightAst);
                return {{
                    id: nextNodeId++,
                    label: '=',
                    type: 'op',
                    isEqualityRoot: true,
                    children: children
                }};
            }} else {{
                const ast = parseTokens(tokenize(clean));
                return ast || {{ id: nextNodeId++, label: clean, type: 'leaf', children: [] }};
            }}
        }}

        function escapeHtml(str) {{
            if (!str) return '';
            return String(str)
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;');
        }}

        function copyCode(id) {{
            const text = document.getElementById(id).textContent;
            navigator.clipboard.writeText(text);
        }}

        const currentPath = window.location.pathname;
        const isSubdir = currentPath.includes('/calculus') || currentPath.includes('/bool') || currentPath.includes('/algebra') || currentPath.includes('/compound');
        const rootPrefix = isSubdir ? '../' : './';
        const linkMain = document.getElementById('demoLinkMain');
        const linkCalc = document.getElementById('demoLinkCalc');
        const linkMulti = document.getElementById('demoLinkMulti');
        const linkBool = document.getElementById('demoLinkBool');
        if (linkMain && linkCalc && linkMulti && linkBool) {{
            linkMain.href = rootPrefix;
            linkCalc.href = rootPrefix + 'calculus/';
            linkMulti.href = rootPrefix + 'algebra/';
            linkBool.href = rootPrefix + 'bool/';
            if (currentPath.includes('/calculus')) {{
                linkCalc.classList.add('active');
            }} else if (currentPath.includes('/algebra') || currentPath.includes('/compound')) {{
                linkMulti.classList.add('active');
            }} else if (currentPath.includes('/bool')) {{
                linkBool.classList.add('active');
            }} else {{
                linkMain.classList.add('active');
            }}
        }}

        resize();
        focusProvenPath();
    </script>
</body>
</html>
"##,
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

    #[test]
    fn test_update_showcase_html() -> Result<(), Box<dyn std::error::Error>> {
        for dir in &[
            "showcase",
            "showcase/algebra",
            "showcase/bool",
            "showcase/calculus",
            "showcase/compound",
            "showcase/deep",
            "showcase/distrib",
            "showcase/super",
            "showcase/trivar",
            "showcase/ultra",
        ] {
            let json_p = format!("{}/trace.json", dir);
            let html_p = format!("{}/index.html", dir);
            if Path::new(&json_p).exists() {
                let json_data = fs::read_to_string(&json_p)?;
                let trace: StaticProofTrace = serde_json::from_str(&json_data)?;
                TraceExporter::export_standalone_html(&trace, Path::new(&html_p))?;
            }
        }
        Ok(())
    }
}
