package gg.salt.hud;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;

/** Default layout, bundled as assets/salthud/default.json (same as the launcher's). */
final class Defaults {
	private Defaults() {}

	static JsonObject json() {
		try (InputStream in = Defaults.class.getResourceAsStream("/assets/salthud/default.json")) {
			return JsonParser.parseReader(new InputStreamReader(in, StandardCharsets.UTF_8)).getAsJsonObject();
		} catch (Exception e) {
			return new JsonObject();
		}
	}
}
