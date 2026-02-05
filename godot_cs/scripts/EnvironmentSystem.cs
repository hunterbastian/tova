using Godot;
using System;

public partial class EnvironmentSystem : Node
{
    private DirectionalLight3D _sun;
    private DirectionalLight3D _bounce;
    private DirectionalLight3D _groundFill;
    private WorldEnvironment _worldEnvironment;

    private float _dayLength = 120f;
    private float _nightLength = 120f;
    private float _cycleLength;
    private float _sunRadius = 220f;
    private float _sunHeight = 140f;

    private Color _skyDay;
    private Color _skyNight;
    private Color _skyGolden;
    private Color _fogDay;
    private Color _fogNight;
    private Color _fogGolden;
    private Color _sunDay;
    private Color _sunNight;
    private Color _sunGolden;

    private string _overrideMode = "";
    public float DayLength => _dayLength;
    public float NightLength => _nightLength;

    public override void _Ready()
    {
        _sun = GetParent().GetNode<DirectionalLight3D>("Sun");
        _bounce = GetParent().GetNode<DirectionalLight3D>("Bounce");
        _groundFill = GetParent().GetNode<DirectionalLight3D>("GroundFill");
        _worldEnvironment = GetParent().GetNode<WorldEnvironment>("WorldEnvironment");

        var config = ConfigLoader.LoadJson("res://data/environment.json");
        _dayLength = ConfigLoader.GetFloat(config, "dayLength", _dayLength);
        _nightLength = ConfigLoader.GetFloat(config, "nightLength", _nightLength);
        _sunRadius = ConfigLoader.GetFloat(config, "sunRadius", _sunRadius);
        _sunHeight = ConfigLoader.GetFloat(config, "sunHeight", _sunHeight);
        _cycleLength = _dayLength + _nightLength;

        _skyDay = ConfigLoader.GetColor(config, "skyDay", new Color("#7db4ff"));
        _skyNight = ConfigLoader.GetColor(config, "skyNight", new Color("#0b1b2e"));
        _skyGolden = ConfigLoader.GetColor(config, "skyGolden", new Color("#f3b06b"));
        _fogDay = ConfigLoader.GetColor(config, "fogDay", new Color("#b8d1f5"));
        _fogNight = ConfigLoader.GetColor(config, "fogNight", new Color("#0a1424"));
        _fogGolden = ConfigLoader.GetColor(config, "fogGolden", new Color("#f1b780"));
        _sunDay = ConfigLoader.GetColor(config, "sunDay", new Color("#ffe2b0"));
        _sunNight = ConfigLoader.GetColor(config, "sunNight", new Color("#4a5a86"));
        _sunGolden = ConfigLoader.GetColor(config, "sunGolden", new Color("#ffb66d"));

        if (_worldEnvironment.Environment == null)
        {
            _worldEnvironment.Environment = new Environment();
        }

        _worldEnvironment.Environment.BackgroundMode = Environment.BGMode.Color;
        _worldEnvironment.Environment.BackgroundColor = _skyDay;
        _worldEnvironment.Environment.FogEnabled = true;
        _worldEnvironment.Environment.FogDensity = 0.0011f;
        _worldEnvironment.Environment.FogLightColor = _fogDay;
        _worldEnvironment.Environment.FogLightEnergy = 0.6f;

        var bounceColor = ConfigLoader.GetColor(config, "bounceColor", new Color("#b7c7ff"));
        var groundColor = ConfigLoader.GetColor(config, "groundFillColor", new Color("#f2d2a8"));
        if (_bounce != null)
        {
            _bounce.LightColor = bounceColor;
        }
        if (_groundFill != null)
        {
            _groundFill.LightColor = groundColor;
        }
    }

    public void SetOverrideMode(string mode)
    {
        if (mode == "day" || mode == "night")
        {
            _overrideMode = mode;
        }
        else
        {
            _overrideMode = "";
        }
    }

    public EnvironmentCycle UpdateCycle(float time, Vector3 playerPosition)
    {
        if (_sun == null) return new EnvironmentCycle();

        var cycleTime = time % _cycleLength;
        var cycle = cycleTime / _cycleLength;
        var dayFraction = _dayLength / _cycleLength;
        var naturalIsDay = cycle < dayFraction;
        var isDay = _overrideMode == "" ? naturalIsDay : _overrideMode == "day";

        float angle;
        float timeToNext;
        if (_overrideMode != "")
        {
            angle = isDay ? Mathf.Pi * 0.5f : Mathf.Pi * 1.5f;
            timeToNext = _cycleLength;
        }
        else if (isDay)
        {
            var dayT = cycle / dayFraction;
            angle = dayT * Mathf.Pi;
            timeToNext = _dayLength - cycleTime;
        }
        else
        {
            var nightT = (cycle - dayFraction) / (1 - dayFraction);
            angle = Mathf.Pi + nightT * Mathf.Pi;
            timeToNext = _cycleLength - cycleTime;
        }

        var sunElevation = Mathf.Sin(angle);
        var daylight = Mathf.Clamp(sunElevation, 0f, 1f);
        var smoothDaylight = Mathf.SmoothStep(0.02f, 0.98f, daylight);

        var sunX = -Mathf.Cos(angle) * _sunRadius;
        var sunY = Mathf.Sin(angle) * _sunHeight + 20f;

        _sun.Position = playerPosition + new Vector3(sunX, sunY, 0f);
        _sun.LookAt(playerPosition, Vector3.Up);

        if (_bounce != null)
        {
            _bounce.Position = playerPosition + new Vector3(-sunX * 0.6f, sunY * 0.35f, -sunX * 0.4f);
            _bounce.LookAt(playerPosition, Vector3.Up);
        }
        if (_groundFill != null)
        {
            _groundFill.Position = playerPosition + new Vector3(sunX * 0.35f, 35f, sunX * 0.2f);
            _groundFill.LookAt(playerPosition, Vector3.Up);
        }

        var goldenRamp = Mathf.SmoothStep(-0.15f, 0.45f, sunElevation);
        var goldenPeak = goldenRamp * (1f - Mathf.SmoothStep(0.45f, 0.9f, sunElevation));
        var goldenBlend = Mathf.Min(1f, goldenPeak * 1.35f);

        var sunColor = _sunNight.Lerp(_sunDay, smoothDaylight).Lerp(_sunGolden, goldenBlend);
        _sun.LightColor = sunColor;
        _sun.LightEnergy = 0.3f + smoothDaylight * 1.45f;

        var skyColor = _skyNight.Lerp(_skyDay, smoothDaylight).Lerp(_skyGolden, goldenBlend);
        var fogColor = _fogNight.Lerp(_fogDay, smoothDaylight).Lerp(_fogGolden, goldenBlend);

        _worldEnvironment.Environment.BackgroundColor = skyColor;
        _worldEnvironment.Environment.FogLightColor = fogColor;

        if (_bounce != null)
        {
            _bounce.LightEnergy = 0.24f + smoothDaylight * 0.34f;
        }
        if (_groundFill != null)
        {
            _groundFill.LightEnergy = 0.18f + smoothDaylight * 0.55f;
        }

        return new EnvironmentCycle
        {
            Daylight = smoothDaylight,
            Night = 1f - smoothDaylight,
            IsDay = isDay,
            CycleTime = cycleTime,
            TimeToNext = timeToNext
        };
    }
}

public struct EnvironmentCycle
{
    public float Daylight;
    public float Night;
    public bool IsDay;
    public float CycleTime;
    public float TimeToNext;
}
