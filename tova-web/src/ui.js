export function createUi({ app, hotbar, safeMode }) {
  const safeModeLabel = safeMode ? "Try enhanced mode" : "Use safe mode";
  const safeModeHref = safeMode ? "?safe=0" : "?safe=1";
  const safeModeHint = safeMode
    ? "Safe mode is now the default for browser reliability."
    : "If the 3D scene looks dark or blank, switch to safe mode.";

  /* Static template — no user input involved, safe to use innerHTML */
  app.innerHTML = [ // eslint-disable-line no-unsanitized/property
    '<div class="shell">',
    '  <div id="veil" class="veil" data-mode="loading">',
    '    <div class="card">',
    '      <p class="eyebrow">Tova Three Build</p>',
    "      <h1>Tova</h1>",
    "      <p>An ash-soft frontier rebuilt around a heavy Three.js renderer with a stronger Morrowind and Oblivion mood.</p>",
    "      <p>Wake on a green rise, claim the shrine sword, cross the pines, and climb toward the old keep beyond the valley haze.</p>",
    '      <div class="controls-grid">',
    "        <div><strong>Click</strong> enter the frontier</div>",
    "        <div><strong>WASD</strong> walk</div>",
    "        <div><strong>Mouse</strong> look</div>",
    "        <div><strong>Enter</strong> walk mode fallback</div>",
    "        <div><strong>Space</strong> jump</div>",
    "        <div><strong>Shift</strong> sprint</div>",
    "        <div><strong>E</strong> take sword</div>",
    "        <div><strong>R</strong> regenerate world</div>",
    "        <div><strong>1-5</strong> hotbar</div>",
    "        <div><strong>Esc</strong> release cursor</div>",
    "      </div>",
    '      <div class="boot-actions">',
    `        <a class="safe-mode-link" href="${safeModeHref}">${safeModeLabel}</a>`,
    `        <span class="safe-mode-hint">${safeModeHint}</span>`,
    "      </div>",
    "    </div>",
    "  </div>",
    '  <div class="hud">',
    '    <div class="compass" id="compass" data-visible="false">',
    '      <div class="compass-notch"></div>',
    '    </div>',
    '    <div id="status" class="status">Three.js frontier loading</div>',
    '    <div id="prompt" class="prompt" data-visible="false"></div>',
    '    <div class="crosshair" id="crosshair" data-visible="false"></div>',
    '    <div id="damage-flash" class="damage-flash"></div>',
    '    <div id="death-screen" class="death-screen" data-visible="false">',
    '      <div class="death-text">You have fallen</div>',
    "    </div>",
    '    <div class="vitals">',
    '      <div class="vital" data-kind="health">',
    '        <div class="vital-label">Health</div>',
    '        <div class="vital-track"><div id="health-fill" class="vital-fill"></div></div>',
    "      </div>",
    '      <div class="vital" data-kind="magicka">',
    '        <div class="vital-label">Magicka</div>',
    '        <div class="vital-track"><div id="magicka-fill" class="vital-fill"></div></div>',
    "      </div>",
    '      <div class="vital" data-kind="fatigue">',
    '        <div class="vital-label">Fatigue</div>',
    '        <div class="vital-track"><div id="fatigue-fill" class="vital-fill"></div></div>',
    "      </div>",
    "    </div>",
    '    <div class="hotbar">',
    '      <div id="selected-label" class="selected-label">Grass</div>',
    '      <div id="hotbar" class="hotbar-row"></div>',
    "    </div>",
    '    <div id="seed" class="seed">Seed 00000000</div>',
    '    <div id="kills" class="kills" data-visible="false">Slain: 0</div>',
    '    <div class="minimap-wrap" id="minimap-wrap" data-visible="false">',
    '      <canvas id="minimap" width="160" height="160"></canvas>',
    "    </div>",
    '    <div class="weapon-panel">',
    '      <div class="weapon-label">Weapon</div>',
    '      <div id="weapon-name" class="weapon-name">Unarmed</div>',
    "    </div>",
    "  </div>",
    "</div>",
  ].join("\n");

  const elements = {
    veil: document.querySelector("#veil"),
    status: document.querySelector("#status"),
    prompt: document.querySelector("#prompt"),
    crosshair: document.querySelector("#crosshair"),
    damageFlash: document.querySelector("#damage-flash"),
    deathScreen: document.querySelector("#death-screen"),
    selectedLabel: document.querySelector("#selected-label"),
    hotbar: document.querySelector("#hotbar"),
    seed: document.querySelector("#seed"),
    kills: document.querySelector("#kills"),
    weaponName: document.querySelector("#weapon-name"),
    healthFill: document.querySelector("#health-fill"),
    magickaFill: document.querySelector("#magicka-fill"),
    fatigueFill: document.querySelector("#fatigue-fill"),
    compass: document.querySelector("#compass"),
    minimapWrap: document.querySelector("#minimap-wrap"),
    minimap: document.querySelector("#minimap"),
  };

  /* ── compass markers ──────────────────────────────────── */
  const COMPASS_RANGE = Math.PI * 0.5; // 90° each side → 180° total visible arc

  const DIRECTIONS = [
    { label: "N",  angle: 0,                  major: true,  north: true },
    { label: "NE", angle: -Math.PI / 4,       major: false, north: false },
    { label: "E",  angle: -Math.PI / 2,       major: true,  north: false },
    { label: "SE", angle: -3 * Math.PI / 4,   major: false, north: false },
    { label: "S",  angle: Math.PI,            major: true,  north: false },
    { label: "SW", angle: 3 * Math.PI / 4,    major: false, north: false },
    { label: "W",  angle: Math.PI / 2,        major: true,  north: false },
    { label: "NW", angle: Math.PI / 4,        major: false, north: false },
  ];

  const POIS = [
    { key: "shrine",  color: "#c4aa69" },
    { key: "castle",  color: "#8c8b84" },
    { key: "forest",  color: "#5a7a4a" },
  ];

  const compassDirEls = DIRECTIONS.map((dir) => {
    const el = document.createElement("span");
    el.className = "compass-dir";
    el.textContent = dir.label;
    if (dir.major) el.dataset.major = "";
    if (dir.north) el.dataset.north = "";
    elements.compass.appendChild(el);
    return { el, angle: dir.angle };
  });

  const compassPoiEls = POIS.map((poi) => {
    const el = document.createElement("span");
    el.className = "compass-poi";
    el.style.background = poi.color;
    elements.compass.appendChild(el);
    return { el, key: poi.key };
  });

  function normalizeAngle(a) {
    while (a > Math.PI) a -= 2 * Math.PI;
    while (a < -Math.PI) a += 2 * Math.PI;
    return a;
  }

  function updateCompass(yaw, playerX, playerZ, landmarks) {
    const half = elements.compass.offsetWidth / 2;
    if (half === 0) return;

    /* ── cardinal / intercardinal direction markers ─────── */
    for (const marker of compassDirEls) {
      const rel = normalizeAngle(marker.angle - yaw);
      if (Math.abs(rel) < COMPASS_RANGE) {
        const px = half - (rel / COMPASS_RANGE) * half;
        marker.el.style.left = `${px}px`;
        marker.el.style.display = "";
      } else {
        marker.el.style.display = "none";
      }
    }

    /* ── point-of-interest markers ──────────────────────── */
    for (const poi of compassPoiEls) {
      const lm = landmarks[poi.key];
      if (!lm) { poi.el.style.display = "none"; continue; }
      const dx = lm.x - playerX;
      const dz = lm.z - playerZ;
      const targetAngle = Math.atan2(-dx, -dz);
      const rel = normalizeAngle(targetAngle - yaw);
      if (Math.abs(rel) < COMPASS_RANGE) {
        const px = half - (rel / COMPASS_RANGE) * half;
        poi.el.style.left = `${px}px`;
        poi.el.style.display = "";
      } else {
        poi.el.style.display = "none";
      }
    }
  }

  /* ── minimap ────────────────────────────────────────── */
  const MINIMAP_SIZE = 160;
  const MINIMAP_WORLD_RANGE = 220; // matches WORLD_SIZE from constants
  const minimapCtx = elements.minimap.getContext("2d");

  const MINIMAP_LANDMARKS = [
    { key: "shrine",  color: "#c4aa69", label: "S" },
    { key: "castle",  color: "#8c8b84", label: "C" },
    { key: "forest",  color: "#5a7a4a", label: "F" },
  ];

  function drawMinimap(yaw, playerX, playerZ, landmarks) {
    const ctx = minimapCtx;
    const half = MINIMAP_SIZE / 2;
    const scale = MINIMAP_SIZE / MINIMAP_WORLD_RANGE;

    ctx.clearRect(0, 0, MINIMAP_SIZE, MINIMAP_SIZE);

    // circular clip
    ctx.save();
    ctx.beginPath();
    ctx.arc(half, half, half - 1, 0, Math.PI * 2);
    ctx.clip();

    // background terrain circle
    ctx.fillStyle = "rgba(9, 8, 7, 0.82)";
    ctx.fill();

    // terrain boundary ring
    ctx.beginPath();
    ctx.arc(half, half, (MINIMAP_WORLD_RANGE / 2) * scale, 0, Math.PI * 2);
    ctx.strokeStyle = "rgba(188, 164, 111, 0.15)";
    ctx.lineWidth = 1;
    ctx.stroke();

    // draw landmarks relative to player, rotated by yaw so "up" = forward
    const cosY = Math.cos(-yaw);
    const sinY = Math.sin(-yaw);

    for (const lm of MINIMAP_LANDMARKS) {
      const pos = landmarks[lm.key];
      if (!pos) continue;

      const dx = pos.x - playerX;
      const dz = pos.z - playerZ;

      // rotate so forward faces up
      const rx = dx * cosY - dz * sinY;
      const rz = dx * sinY + dz * cosY;

      const sx = half + rx * scale;
      const sy = half - rz * scale;

      // skip if outside circle
      const fromCenter = Math.sqrt((sx - half) ** 2 + (sy - half) ** 2);
      if (fromCenter > half - 4) continue;

      // diamond marker
      ctx.save();
      ctx.translate(sx, sy);
      ctx.rotate(Math.PI / 4);
      ctx.fillStyle = lm.color;
      ctx.fillRect(-3.5, -3.5, 7, 7);
      ctx.restore();

      // label
      ctx.fillStyle = "rgba(230, 220, 199, 0.7)";
      ctx.font = "bold 8px sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(lm.label, sx, sy - 7);
    }

    // spawn marker (origin)
    {
      const dx = 0 - playerX;
      const dz = 0 - playerZ;
      const rx = dx * cosY - dz * sinY;
      const rz = dx * sinY + dz * cosY;
      const sx = half + rx * scale;
      const sy = half - rz * scale;
      const fromCenter = Math.sqrt((sx - half) ** 2 + (sy - half) ** 2);
      if (fromCenter < half - 4) {
        ctx.beginPath();
        ctx.arc(sx, sy, 3, 0, Math.PI * 2);
        ctx.fillStyle = "rgba(143, 163, 88, 0.6)";
        ctx.fill();
      }
    }

    // player arrow at center
    ctx.save();
    ctx.translate(half, half);
    ctx.fillStyle = "#e6dcc7";
    ctx.beginPath();
    ctx.moveTo(0, -6);
    ctx.lineTo(4, 5);
    ctx.lineTo(0, 2);
    ctx.lineTo(-4, 5);
    ctx.closePath();
    ctx.fill();
    ctx.restore();

    // north indicator — small "N" at the edge of the circle toward north
    {
      const northAngle = -yaw - Math.PI / 2;
      const nx = half + Math.cos(northAngle) * (half - 10);
      const ny = half + Math.sin(northAngle) * (half - 10);
      ctx.fillStyle = "#c4aa69";
      ctx.font = "bold 9px sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText("N", nx, ny);
    }

    ctx.restore(); // pop circular clip
  }

  for (const [index, slot] of hotbar.entries()) {
    const element = document.createElement("div");
    element.className = "slot";
    element.dataset.index = String(index);

    const indexSpan = document.createElement("span");
    indexSpan.className = "slot-index";
    indexSpan.textContent = String(index + 1);
    element.appendChild(indexSpan);

    const chipSpan = document.createElement("span");
    chipSpan.className = "slot-chip";
    chipSpan.style.background = slot.color;
    element.appendChild(chipSpan);

    elements.hotbar.appendChild(element);
  }

  function setStatus(state, message) {
    state.status = message;
    state.lastStatusAt = performance.now();
    elements.status.textContent = message;
  }

  function flashDamage() {
    elements.damageFlash.classList.remove("active");
    void elements.damageFlash.offsetWidth;
    elements.damageFlash.classList.add("active");
  }

  function showDeathScreen(visible) {
    elements.deathScreen.dataset.visible = String(visible);
  }

  function updateHud({ state, controlsLocked, canControl, interactionPrompt, compass }) {
    elements.selectedLabel.textContent = hotbar[state.selectedSlot].label;
    elements.seed.textContent = `Seed ${state.seed.toString(16).padStart(8, "0").toUpperCase()}`;
    elements.weaponName.textContent = state.hasSword ? "Iron Sword" : "Unarmed";
    elements.crosshair.dataset.visible = String(canControl() && !state.isDead);

    const compassVisible = canControl() && !state.isDead;
    elements.compass.dataset.visible = String(compassVisible);
    elements.minimapWrap.dataset.visible = String(compassVisible);
    if (compassVisible && compass) {
      updateCompass(compass.yaw, compass.playerX, compass.playerZ, compass.landmarks);
      drawMinimap(compass.yaw, compass.playerX, compass.playerZ, compass.landmarks);
    }

    elements.veil.dataset.mode = state.mode === "ready" ? "ready" : "loading";
    elements.healthFill.style.transform = `scaleX(${state.health})`;
    elements.magickaFill.style.transform = `scaleX(${state.magicka})`;
    elements.fatigueFill.style.transform = `scaleX(${state.fatigue})`;

    if (state.kills > 0) {
      elements.kills.dataset.visible = "true";
      elements.kills.textContent = `Slain: ${state.kills}`;
    } else {
      elements.kills.dataset.visible = "false";
    }

    state.interactionPrompt = interactionPrompt;
    elements.prompt.textContent = state.interactionPrompt;
    elements.prompt.dataset.visible = String(Boolean(state.interactionPrompt));

    for (const child of elements.hotbar.children) {
      child.dataset.active = String(Number(child.dataset.index) === state.selectedSlot);
    }

    if (performance.now() - state.lastStatusAt > 3000 && !controlsLocked) {
      elements.status.textContent = "Click or press Enter to enter the frontier";
    }
  }

  return { elements, flashDamage, setStatus, showDeathScreen, updateHud };
}
