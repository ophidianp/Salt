package gg.salt.config;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.fabricmc.loader.api.FabricLoader;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * The launcher's look, from config/salt-theme.json. The launcher writes it
 * before launch and again whenever its settings change, so it's re-read
 * (cheaply, by modified time) every couple of seconds while the game runs.
 */
public final class SaltTheme {
	private static final int DEFAULT_ACCENT = 0xFF63A7FF;
	private static final long POLL_NANOS = 2_000_000_000L;

	private static int accent = DEFAULT_ACCENT;
	private static int radius = 4;
	private static long lastPoll, lastModified = -1;

	private SaltTheme() {}

	private static Path path() {
		return FabricLoader.getInstance().getConfigDir().resolve("salt-theme.json");
	}

	private static void poll() {
		long now = System.nanoTime();
		if (lastModified != -1 && now - lastPoll < POLL_NANOS) return;
		lastPoll = now;
		try {
			Path p = path();
			long m = Files.exists(p) ? Files.getLastModifiedTime(p).toMillis() : 0;
			if (m == lastModified) return;
			lastModified = m;
			if (m == 0) return;
			JsonObject o = JsonParser.parseString(Files.readString(p, StandardCharsets.UTF_8)).getAsJsonObject();
			if (o.has("accent")) accent = parse(o.get("accent").getAsString(), accent);
			// The launcher's radius is CSS px (~14); GUI pixels are ~3-4x bigger.
			if (o.has("radius")) radius = Math.max(0, Math.min(6, Math.round(o.get("radius").getAsInt() / 3.5f)));
		} catch (Exception e) {
			lastModified = 0;
			SaltConfig.LOGGER.warn("saltConfig: couldn't read salt-theme.json: {}", e.toString());
		}
	}

	/** The launcher's accent colour (opaque ARGB). */
	public static int accent() {
		poll();
		return accent;
	}

	/** Corner radius for Salt surfaces, in GUI pixels. */
	public static int radius() {
		poll();
		return radius;
	}

	/** "#rgb", "#rrggbb" or "#aarrggbb" → opaque ARGB. */
	static int parse(String s, int def) {
		try {
			String h = s.trim().startsWith("#") ? s.trim().substring(1) : s.trim();
			if (h.length() == 3) h = "" + h.charAt(0) + h.charAt(0) + h.charAt(1) + h.charAt(1) + h.charAt(2) + h.charAt(2);
			if (h.length() == 8) h = h.substring(2);
			if (h.length() != 6) return def;
			return 0xFF000000 | Integer.parseInt(h, 16);
		} catch (NumberFormatException e) {
			return def;
		}
	}
}
