package gg.salt.config;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.blaze3d.platform.InputConstants;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.KeyMapping;
import net.minecraft.resources.Identifier;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * saltConfig — one settings screen for every Salt mod (like OneConfig).
 * Mods register a page with {@link #mod}; Right Shift opens the screen.
 * Values live in config/saltconfig.json.
 */
public class SaltConfig implements ClientModInitializer {
	public static final Logger LOGGER = LoggerFactory.getLogger("saltConfig");

	private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
	private static final List<ModConfig> MODS = new ArrayList<>();
	private static JsonObject saved;
	private static boolean dirty;
	private static KeyMapping openKey;

	/** Registers (or returns the existing) page for a mod. */
	public static synchronized ModConfig mod(String id, String name, String description) {
		for (ModConfig m : MODS) if (m.id.equals(id)) return m;
		ModConfig m = new ModConfig(id, name, description);
		MODS.add(m);
		// Saved values are applied once the page's options are declared;
		// see applySaved(), called lazily before first use.
		return m;
	}

	public static List<ModConfig> mods() {
		for (ModConfig m : MODS) m.load();
		return Collections.unmodifiableList(MODS);
	}

	/** The key mapping category shared by all Salt mods. */
	public static KeyMapping.Category category() {
		Identifier id = Identifier.fromNamespaceAndPath("salt", "salt");
		try {
			return KeyMapping.Category.register(id);
		} catch (IllegalArgumentException alreadyRegistered) {
			return new KeyMapping.Category(id);
		}
	}

	private static final boolean SELF_TEST = Boolean.getBoolean("saltconfig.selftest");
	private static boolean selfTested;

	@Override
	public void onInitializeClient() {
		openKey = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.saltconfig.open", InputConstants.Type.KEYSYM, InputConstants.KEY_RSHIFT, category()));
		ClientTickEvents.END_CLIENT_TICK.register(mc -> {
			while (openKey.consumeClick()) {
				if (mc.screen == null) mc.setScreen(new SaltConfigScreen(null));
			}
			if (dirty) save();
			// Dev check (-Dsaltconfig.selftest=true): once the title screen is up, log every mod's inline page.
			if (SELF_TEST && !selfTested && mc.screen != null) {
				selfTested = true;
				new SaltConfigScreen(null).selfTest();
			}
		});
		LOGGER.info("saltConfig ready");
	}

	// --------------------------------------------------------- persistence

	private static Path path() {
		return FabricLoader.getInstance().getConfigDir().resolve("saltconfig.json");
	}

	private static JsonObject saved() {
		if (saved == null) {
			try {
				saved = Files.exists(path()) ? JsonParser.parseString(Files.readString(path(), StandardCharsets.UTF_8)).getAsJsonObject() : new JsonObject();
			} catch (Exception e) {
				LOGGER.warn("saltConfig: couldn't read saltconfig.json", e);
				saved = new JsonObject();
			}
		}
		return saved;
	}

	/**
	 * Copies a page's saved values in. Called lazily the first time the page
	 * is read, by which point the mod has declared all of its options.
	 */
	static synchronized void applySaved(ModConfig m) {
		JsonObject all = saved();
		{
			if (!all.has(m.id)) return;
			JsonObject o = all.getAsJsonObject(m.id);
			if (o.has("enabled")) m.enabled = o.get("enabled").getAsBoolean();
			JsonObject vals = o.has("values") ? o.getAsJsonObject("values") : new JsonObject();
			for (Option opt : m.options) {
				String key = switch (opt) {
					case Option.Toggle t -> t.key();
					case Option.Slider s -> s.key();
					case Option.Choice c -> c.key();
					case Option.Action a -> null;
				};
				if (key == null || !vals.has(key)) continue;
				JsonElement v = vals.get(key);
				try {
					switch (opt) {
						case Option.Toggle t -> m.values.put(key, v.getAsBoolean());
						case Option.Slider s -> m.values.put(key, Math.max(s.min(), Math.min(s.max(), v.getAsDouble())));
						case Option.Choice c -> { if (c.choices().contains(v.getAsString())) m.values.put(key, v.getAsString()); }
						case Option.Action a -> {}
					}
				} catch (RuntimeException ignored) {
					// Wrong type in the file: keep the default.
				}
			}
		}
	}

	static void markDirty() {
		dirty = true;
	}

	static synchronized void save() {
		dirty = false;
		JsonObject all = saved();
		for (ModConfig m : MODS) {
			m.load(); // never overwrite saved values with defaults
			JsonObject o = new JsonObject();
			o.addProperty("enabled", m.enabled);
			JsonObject vals = new JsonObject();
			m.values.forEach((k, v) -> {
				if (v instanceof Boolean b) vals.addProperty(k, b);
				else if (v instanceof Number n) vals.addProperty(k, n);
				else vals.addProperty(k, String.valueOf(v));
			});
			o.add("values", vals);
			all.add(m.id, o);
		}
		try {
			Files.createDirectories(path().getParent());
			Files.writeString(path(), GSON.toJson(all), StandardCharsets.UTF_8);
		} catch (Exception e) {
			LOGGER.warn("saltConfig: couldn't save saltconfig.json", e);
		}
	}
}
