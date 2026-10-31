package gg.salt.config;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * One mod's page in saltConfig. Built fluently by the mod at startup:
 *
 * <pre>
 * static final ModConfig CFG = SaltConfig.mod("saltzoom", "saltZoom", "Smooth zoom")
 *     .toggle("smooth", "Smooth zoom", "Ease in and out", true)
 *     .slider("zoom", "Default zoom", "", 1.5, 10, 0.5, 4);
 * ...
 * if (CFG.enabled() &amp;&amp; CFG.bool("smooth")) ...
 * </pre>
 *
 * Every mod also gets an "Enabled" switch at the top of its page.
 */
public final class ModConfig {
	public final String id, name, description;
	final List<Option> options = new ArrayList<>();
	final Map<String, Object> values = new LinkedHashMap<>();
	boolean enabled = true;
	private boolean loaded;

	/** Applies saved values once, on first read (after all options are declared). */
	void load() {
		if (!loaded) {
			loaded = true;
			SaltConfig.applySaved(this);
		}
	}

	ModConfig(String id, String name, String description) {
		this.id = id;
		this.name = name;
		this.description = description;
	}

	// ------------------------------------------------------------ building

	public ModConfig toggle(String key, String label, String description, boolean def) {
		options.add(new Option.Toggle(key, label, description, def));
		values.putIfAbsent(key, def);
		return this;
	}

	public ModConfig slider(String key, String label, String description, double min, double max, double step, double def) {
		options.add(new Option.Slider(key, label, description, min, max, step, def));
		values.putIfAbsent(key, def);
		return this;
	}

	public ModConfig choice(String key, String label, String description, List<String> choices, String def) {
		options.add(new Option.Choice(key, label, description, choices, def));
		values.putIfAbsent(key, def);
		return this;
	}

	/** A button on the page, e.g. "Edit HUD layout". */
	public ModConfig action(String label, String description, Runnable run) {
		options.add(new Option.Action(label, description, run));
		return this;
	}

	// ------------------------------------------------------------- reading

	public boolean enabled() {
		load();
		return enabled;
	}

	public boolean bool(String key) {
		load();
		return values.get(key) instanceof Boolean b && b;
	}

	public double number(String key) {
		load();
		return values.get(key) instanceof Number n ? n.doubleValue() : 0;
	}

	public String choice(String key) {
		load();
		return values.get(key) instanceof String s ? s : "";
	}

	public void set(String key, Object value) {
		load();
		values.put(key, value);
		SaltConfig.markDirty();
	}

	public void setEnabled(boolean on) {
		load();
		enabled = on;
		SaltConfig.markDirty();
	}
}
