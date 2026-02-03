import * as THREE from 'three';

export class Environment {
    constructor(scene, options = {}) {
        this.scene = scene;
        this.enableShadows = options.enableShadows ?? true;
        this.dayLength = options.dayLength ?? 30;
        this.nightLength = options.nightLength ?? 30;
        this.cycleLength = this.dayLength + this.nightLength;

        this.colors = {
            skyDay: new THREE.Color(0x87ceeb),
            skyNight: new THREE.Color(0x0b1b2e),
            fogDay: new THREE.Color(0xcfe3ff),
            fogNight: new THREE.Color(0x0a1424),
            sunDay: new THREE.Color(0xfff1d0),
            sunNight: new THREE.Color(0x4a5a86),
            skyGolden: new THREE.Color(0xf7b46a),
            fogGolden: new THREE.Color(0xf2b178),
            sunGolden: new THREE.Color(0xffb15e)
        };

        this.ambientLight = null;
        this.hemiLight = null;
        this.sunLight = null;
        this.overrideMode = null;
        this.init();
    }

    init() {
        // Sky color (gradient or solid for now)
        this.scene.background = this.colors.skyDay.clone();

        // Fog for depth and atmosphere
        this.scene.fog = new THREE.FogExp2(this.colors.fogDay.clone(), 0.0015);

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
            this.sunLight.shadow.radius = 3;
            this.sunLight.shadow.bias = -0.0005;
        }

        this.scene.add(this.sunLight);
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
        if (isDay) {
            const dayT = cycle / dayFraction; // 0..1
            angle = dayT * Math.PI; // sunrise -> sunset
        } else {
            const nightT = (cycle - dayFraction) / (1 - dayFraction);
            angle = Math.PI + nightT * Math.PI; // below horizon
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

        const goldenRamp = THREE.MathUtils.smoothstep(sunElevation, -0.15, 0.45);
        const goldenPeak = goldenRamp * (1 - THREE.MathUtils.smoothstep(sunElevation, 0.45, 0.9));
        const goldenBlend = Math.min(1, goldenPeak * 1.35);

        const sunColor = this.colors.sunNight
            .clone()
            .lerp(this.colors.sunDay, smoothDaylight)
            .lerp(this.colors.sunGolden, goldenBlend);
        this.sunLight.color.copy(sunColor);
        this.sunLight.intensity = 0.2 + smoothDaylight * 1.25;

        const skyColor = this.colors.skyNight
            .clone()
            .lerp(this.colors.skyDay, smoothDaylight)
            .lerp(this.colors.skyGolden, goldenBlend);
        const fogColor = this.colors.fogNight
            .clone()
            .lerp(this.colors.fogDay, smoothDaylight)
            .lerp(this.colors.fogGolden, goldenBlend);
        this.scene.background.copy(skyColor);
        if (this.scene.fog) {
            this.scene.fog.color.copy(fogColor);
        }

        this.ambientLight.intensity = 0.28 + smoothDaylight * 0.32;
        this.hemiLight.intensity = 0.32 + smoothDaylight * 0.4;
        this.hemiLight.color.copy(skyColor.clone().lerp(new THREE.Color(0xffffff), 0.35));
        this.hemiLight.groundColor.copy(
            new THREE.Color(0x0b0f1a).lerp(new THREE.Color(0x2c3b2b), smoothDaylight)
        );

        const timeToNext = isDay
            ? this.dayLength - cycleTime
            : this.cycleLength - cycleTime;

        return {
            daylight: smoothDaylight,
            night: 1 - smoothDaylight,
            isDay,
            cycleTime,
            timeToNext
        };
    }
}
