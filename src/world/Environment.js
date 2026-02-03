import * as THREE from 'three';

export class Environment {
    constructor(scene, options = {}) {
        this.scene = scene;
        this.enableShadows = options.enableShadows ?? true;
        this.dayLength = options.dayLength ?? 30;
        this.nightLength = options.nightLength ?? 30;
        this.cycleLength = this.dayLength + this.nightLength;

        this.colors = {
            skyDay: new THREE.Color(0x7db4ff),
            skyNight: new THREE.Color(0x0b1b2e),
            fogDay: new THREE.Color(0xb8d1f5),
            fogNight: new THREE.Color(0x0a1424),
            sunDay: new THREE.Color(0xffe2b0),
            sunNight: new THREE.Color(0x4a5a86),
            skyGolden: new THREE.Color(0xf3b06b),
            fogGolden: new THREE.Color(0xf1b780),
            sunGolden: new THREE.Color(0xffb66d)
        };

        this._sunColor = new THREE.Color();
        this._skyColor = new THREE.Color();
        this._fogColor = new THREE.Color();
        this._hemiColor = new THREE.Color();
        this._groundColor = new THREE.Color();
        this._groundDay = new THREE.Color(0x2c3b2b);
        this._groundNight = new THREE.Color(0x0b0f1a);
        this._white = new THREE.Color(0xffffff);

        this.ambientLight = null;
        this.hemiLight = null;
        this.sunLight = null;
        this.bounceLight = null;
        this.groundFill = null;
        this.overrideMode = null;
        this.init();
    }

    init() {
        // Sky color (gradient or solid for now)
        this.scene.background = this.colors.skyDay.clone();

        // Fog for depth and atmosphere
        this.scene.fog = new THREE.FogExp2(this.colors.fogDay.clone(), 0.0011);

        // Lighting
        this.setupLights();
    }

    setupLights() {
        this.ambientLight = new THREE.AmbientLight(0x404040, 0.5);
        this.scene.add(this.ambientLight);

        this.hemiLight = new THREE.HemisphereLight(0xffffff, 0x34405f, 0.6);
        this.hemiLight.position.set(0, 200, 0);
        this.scene.add(this.hemiLight);

        this.sunLight = new THREE.DirectionalLight(this.colors.sunDay.clone(), 1.3);
        this.sunLight.position.set(100, 100, 50);
        this.sunLight.castShadow = this.enableShadows;

        // Shadow properties tuned for a balance of quality and performance.
        // On many Macs, a 1024x1024 shadow map is a good compromise between
        // visual quality and framerate.
        if (this.enableShadows) {
            this.sunLight.shadow.mapSize.width = 512;
            this.sunLight.shadow.mapSize.height = 512;
            this.sunLight.shadow.camera.near = 0.5;
            this.sunLight.shadow.camera.far = 350;

            const d = 160;
            this.sunLight.shadow.camera.left = -d;
            this.sunLight.shadow.camera.right = d;
            this.sunLight.shadow.camera.top = d;
            this.sunLight.shadow.camera.bottom = -d;

            // Soft shadows
            this.sunLight.shadow.radius = 4;
            this.sunLight.shadow.bias = -0.0005;
        }

        this.scene.add(this.sunLight);

        // Soft cool fill to keep shadows readable (Sildur-like light wrap).
        this.bounceLight = new THREE.DirectionalLight(0xb7c7ff, 0.35);
        this.bounceLight.position.set(-80, 60, -60);
        this.bounceLight.castShadow = false;
        this.scene.add(this.bounceLight);

        // Terrain-only warm fill to lift ground brightness without washing the sky.
        this.groundFill = new THREE.DirectionalLight(0xf2d2a8, 0.45);
        this.groundFill.position.set(60, 30, 120);
        this.groundFill.castShadow = false;
        this.groundFill.layers.enable(1);
        this.scene.add(this.groundFill);
    }

    setOverrideMode(mode) {
        this.overrideMode = mode === 'day' || mode === 'night' ? mode : null;
    }

    update(time, playerPosition) {
        if (!this.sunLight) return;

        const cycleTime = time % this.cycleLength;
        const cycle = cycleTime / this.cycleLength;
        const dayFraction = this.dayLength / this.cycleLength;
        const naturalIsDay = cycle < dayFraction;
        const isDay = this.overrideMode
            ? this.overrideMode === 'day'
            : naturalIsDay;

        let angle;
        let timeToNext;
        if (this.overrideMode) {
            // Force a stable midday / midnight position when overridden.
            angle = isDay ? Math.PI * 0.5 : Math.PI * 1.5;
            timeToNext = this.cycleLength;
        } else if (isDay) {
            const dayT = cycle / dayFraction; // 0..1
            angle = dayT * Math.PI; // sunrise -> sunset
            timeToNext = this.dayLength - cycleTime;
        } else {
            const nightT = (cycle - dayFraction) / (1 - dayFraction);
            angle = Math.PI + nightT * Math.PI; // below horizon
            timeToNext = this.cycleLength - cycleTime;
        }

        const sunElevation = Math.sin(angle);
        const daylight = THREE.MathUtils.clamp(sunElevation, 0, 1);
        const smoothDaylight = THREE.MathUtils.smoothstep(daylight, 0.02, 0.98);

        // Sun moves east (-x) to west (+x) over the player.
        const sunRadius = 220;
        const sunHeight = 140;
        const sunX = -Math.cos(angle) * sunRadius;
        const sunY = Math.sin(angle) * sunHeight + 20;

        this.sunLight.position.set(
            playerPosition.x + sunX,
            playerPosition.y + sunY,
            playerPosition.z
        );
        this.sunLight.target.position.set(playerPosition.x, playerPosition.y, playerPosition.z);
        this.sunLight.target.updateMatrixWorld();
        if (this.bounceLight) {
            this.bounceLight.position.set(
                playerPosition.x - sunX * 0.6,
                playerPosition.y + sunY * 0.35,
                playerPosition.z - sunX * 0.4
            );
        }
        if (this.groundFill) {
            this.groundFill.position.set(
                playerPosition.x + sunX * 0.35,
                playerPosition.y + 35,
                playerPosition.z + sunX * 0.2
            );
        }

        const goldenRamp = THREE.MathUtils.smoothstep(sunElevation, -0.15, 0.45);
        const goldenPeak = goldenRamp * (1 - THREE.MathUtils.smoothstep(sunElevation, 0.45, 0.9));
        const goldenBlend = Math.min(1, goldenPeak * 1.35);

        const sunColor = this._sunColor
            .copy(this.colors.sunNight)
            .lerp(this.colors.sunDay, smoothDaylight)
            .lerp(this.colors.sunGolden, goldenBlend);
        this.sunLight.color.copy(sunColor);
        this.sunLight.intensity = 0.3 + smoothDaylight * 1.45;

        const skyColor = this._skyColor
            .copy(this.colors.skyNight)
            .lerp(this.colors.skyDay, smoothDaylight)
            .lerp(this.colors.skyGolden, goldenBlend);
        const fogColor = this._fogColor
            .copy(this.colors.fogNight)
            .lerp(this.colors.fogDay, smoothDaylight)
            .lerp(this.colors.fogGolden, goldenBlend);
        this.scene.background.copy(skyColor);
        if (this.scene.fog) {
            this.scene.fog.color.copy(fogColor);
        }

        this.ambientLight.intensity = 0.3 + smoothDaylight * 0.28;
        this.hemiLight.intensity = 0.42 + smoothDaylight * 0.5;
        this.hemiLight.color.copy(this._hemiColor.copy(skyColor).lerp(this._white, 0.35));
        this.hemiLight.groundColor.copy(
            this._groundColor.copy(this._groundNight).lerp(this._groundDay, smoothDaylight)
        );
        if (this.bounceLight) {
            this.bounceLight.intensity = 0.24 + smoothDaylight * 0.34;
        }
        if (this.groundFill) {
            this.groundFill.intensity = 0.18 + smoothDaylight * 0.55;
        }

        return {
            daylight: smoothDaylight,
            night: 1 - smoothDaylight,
            isDay,
            cycleTime,
            timeToNext
        };
    }
}
