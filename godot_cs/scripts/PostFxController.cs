using Godot;

public partial class PostFxController : Node
{
    private WorldEnvironment _worldEnvironment;
    private bool _enableVignette = true;
    private bool _enableGrain = true;

    public override void _Ready()
    {
        _worldEnvironment = GetParent().GetNode<WorldEnvironment>("WorldEnvironment");
        var config = ConfigLoader.LoadJson("res://data/ui.json");
        _enableVignette = ConfigLoader.GetBool(config, "enableVignette", true);
        _enableGrain = ConfigLoader.GetBool(config, "enableGrain", true);

        if (_worldEnvironment.Environment == null)
        {
            _worldEnvironment.Environment = new Environment();
        }

        var env = _worldEnvironment.Environment;
        env.TonemapMode = Environment.ToneMapper.Aces;
        env.TonemapExposure = 0.9f;
        env.GlowEnabled = true;
        env.GlowStrength = 0.35f;
        env.GlowBloom = 0.6f;
        env.GlowBlendMode = Environment.GlowBlendModeEnum.Softlight;
        env.GlowHdrThreshold = 0.3f;
        env.GlowLevels = 2;

        if (_enableVignette || _enableGrain)
        {
            CreateOverlay();
        }
    }

    private void CreateOverlay()
    {
        var layer = new CanvasLayer();
        layer.Layer = 50;
        var rect = new ColorRect();
        rect.AnchorRight = 1f;
        rect.AnchorBottom = 1f;
        rect.SizeFlagsHorizontal = Control.SizeFlags.ExpandFill;
        rect.SizeFlagsVertical = Control.SizeFlags.ExpandFill;

        var shader = new Shader();
        shader.Code = @"shader_type canvas_item;
uniform sampler2D screen_texture : hint_screen_texture, repeat_disable, filter_linear;
uniform bool enable_vignette = true;
uniform bool enable_grain = true;
uniform float vignette_strength = 0.6;
uniform float grain_strength = 0.06;
void fragment() {
    vec2 uv = SCREEN_UV;
    vec4 col = texture(screen_texture, uv);
    if (enable_vignette) {
        vec2 dist = uv - vec2(0.5, 0.45);
        float d = length(dist);
        float vignette = smoothstep(0.65, 0.95, d);
        col.rgb *= mix(1.0, 0.55, vignette * vignette_strength);
    }
    if (enable_grain) {
        float noise = fract(sin(dot(uv * vec2(128.0, 128.0) + TIME, vec2(12.9898,78.233))) * 43758.5453);
        col.rgb += (noise - 0.5) * grain_strength;
    }
    COLOR = col;
}";

        var mat = new ShaderMaterial();
        mat.Shader = shader;
        mat.SetShaderParameter("enable_vignette", _enableVignette);
        mat.SetShaderParameter("enable_grain", _enableGrain);
        rect.Material = mat;

        layer.AddChild(rect);
        GetTree().Root.AddChild(layer);
    }
}
