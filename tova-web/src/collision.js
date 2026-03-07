const PLAYER_RADIUS = 0.35;

export function createCollisionSystem() {
  const colliders = [];

  function addCylinder(x, z, radius) {
    colliders.push({ t: 0, x, z, r: radius });
  }

  function addBox(cx, cz, halfW, halfD) {
    colliders.push({ t: 1, x: cx, z: cz, hw: halfW, hd: halfD });
  }

  function clear() {
    colliders.length = 0;
  }

  function resolve(pos) {
    for (let i = 0; i < colliders.length; i++) {
      const c = colliders[i];

      if (c.t === 0) {
        /* ── cylinder: circle-vs-circle in xz ──────────── */
        const dx = pos.x - c.x;
        const dz = pos.z - c.z;
        const distSq = dx * dx + dz * dz;
        const minDist = c.r + PLAYER_RADIUS;
        if (distSq < minDist * minDist && distSq > 0.0001) {
          const dist = Math.sqrt(distSq);
          const push = (minDist - dist) / dist;
          pos.x += dx * push;
          pos.z += dz * push;
        }
      } else {
        /* ── box: AABB closest-point test ───────────────── */
        const nearX = Math.max(c.x - c.hw, Math.min(pos.x, c.x + c.hw));
        const nearZ = Math.max(c.z - c.hd, Math.min(pos.z, c.z + c.hd));
        const dx = pos.x - nearX;
        const dz = pos.z - nearZ;
        const distSq = dx * dx + dz * dz;

        if (distSq < PLAYER_RADIUS * PLAYER_RADIUS) {
          if (distSq > 0.0001) {
            const dist = Math.sqrt(distSq);
            const push = (PLAYER_RADIUS - dist) / dist;
            pos.x += dx * push;
            pos.z += dz * push;
          } else {
            /* player center inside box — push to nearest edge */
            const toMinX = pos.x - (c.x - c.hw);
            const toMaxX = c.x + c.hw - pos.x;
            const toMinZ = pos.z - (c.z - c.hd);
            const toMaxZ = c.z + c.hd - pos.z;
            const min = Math.min(toMinX, toMaxX, toMinZ, toMaxZ);
            if (min === toMinX) pos.x = c.x - c.hw - PLAYER_RADIUS;
            else if (min === toMaxX) pos.x = c.x + c.hw + PLAYER_RADIUS;
            else if (min === toMinZ) pos.z = c.z - c.hd - PLAYER_RADIUS;
            else pos.z = c.z + c.hd + PLAYER_RADIUS;
          }
        }
      }
    }
  }

  return { addCylinder, addBox, clear, resolve };
}
