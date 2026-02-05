using Godot;
using System;
using System.Collections.Generic;

public static class ConfigLoader
{
    public static Godot.Collections.Dictionary LoadJson(string path)
    {
        if (!FileAccess.FileExists(path))
        {
            GD.PushWarning($"Config not found: {path}");
            return new Godot.Collections.Dictionary();
        }

        using var file = FileAccess.Open(path, FileAccess.ModeFlags.Read);
        var jsonText = file.GetAsText();
        var parsed = Json.ParseString(jsonText);
        if (parsed.VariantType == Variant.Type.Dictionary)
        {
            return (Godot.Collections.Dictionary)parsed;
        }

        GD.PushWarning($"Config parse failed: {path}");
        return new Godot.Collections.Dictionary();
    }

    public static float GetFloat(Godot.Collections.Dictionary dict, string key, float fallback)
    {
        if (!dict.ContainsKey(key)) return fallback;
        return Convert.ToSingle(dict[key]);
    }

    public static int GetInt(Godot.Collections.Dictionary dict, string key, int fallback)
    {
        if (!dict.ContainsKey(key)) return fallback;
        return Convert.ToInt32(dict[key]);
    }

    public static bool GetBool(Godot.Collections.Dictionary dict, string key, bool fallback)
    {
        if (!dict.ContainsKey(key)) return fallback;
        return Convert.ToBoolean(dict[key]);
    }

    public static string GetString(Godot.Collections.Dictionary dict, string key, string fallback)
    {
        if (!dict.ContainsKey(key)) return fallback;
        return dict[key].ToString();
    }

    public static Color GetColor(Godot.Collections.Dictionary dict, string key, Color fallback)
    {
        if (!dict.ContainsKey(key)) return fallback;
        var text = dict[key].ToString();
        if (text.StartsWith("#"))
        {
            return new Color(text);
        }
        return fallback;
    }

    public static Godot.Collections.Dictionary GetDict(Godot.Collections.Dictionary dict, string key)
    {
        if (!dict.ContainsKey(key)) return new Godot.Collections.Dictionary();
        if (dict[key].VariantType == Variant.Type.Dictionary)
        {
            return (Godot.Collections.Dictionary)dict[key];
        }
        return new Godot.Collections.Dictionary();
    }
}
