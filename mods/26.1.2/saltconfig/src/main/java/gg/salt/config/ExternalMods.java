package gg.salt.config;

import com.terraformersmc.modmenu.api.ConfigScreenFactory;
import com.terraformersmc.modmenu.api.ModMenuApi;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;
import net.fabricmc.loader.api.metadata.CustomValue;
import net.fabricmc.loader.api.metadata.ModMetadata;
import net.fabricmc.loader.api.metadata.Person;
import net.minecraft.client.gui.screens.Screen;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.function.Function;
import java.util.stream.Collectors;

/**
 * Every other installed mod, for saltConfig's "Mods" list (it replaces Mod
 * Menu). Config screens come from the standard {@code "modmenu"} entrypoint,
 * which is how Cloth Config, YACL and most other config libraries expose them.
 */
public final class ExternalMods {
	public record Mod(String id, String name, String version, String authors, String description, boolean library, Function<Screen, Screen> openConfig) {}

	private static final Set<String> HIDDEN = Set.of("java", "minecraft", "fabricloader", "mixinextras");
	private static List<Mod> mods;

	private ExternalMods() {}

	public static synchronized List<Mod> all() {
		if (mods == null) mods = scan();
		return mods;
	}

	private static List<Mod> scan() {
		Map<String, ConfigScreenFactory<?>> factories = new HashMap<>();
		for (var c : FabricLoader.getInstance().getEntrypointContainers("modmenu", ModMenuApi.class)) {
			String owner = c.getProvider().getMetadata().getId();
			try {
				ModMenuApi api = c.getEntrypoint();
				ConfigScreenFactory<?> f = api.getModConfigScreenFactory();
				if (f != null) factories.putIfAbsent(owner, f);
				api.getProvidedConfigScreenFactories().forEach(factories::putIfAbsent);
			} catch (Throwable t) {
				SaltConfig.LOGGER.warn("Couldn't read the config screen of {}: {}", owner, t.toString());
			}
		}

		Set<String> salt = SaltConfig.mods().stream().map(m -> m.id).collect(Collectors.toSet());
		List<Mod> out = new ArrayList<>();
		for (ModContainer c : FabricLoader.getInstance().getAllMods()) {
			ModMetadata meta = c.getMetadata();
			String id = meta.getId();
			if (c.getContainingMod().isPresent() || HIDDEN.contains(id) || "builtin".equals(meta.getType())) continue;
			if (id.equals("saltconfig") || salt.contains(id)) continue;
			String authors = meta.getAuthors().stream().map(Person::getName).collect(Collectors.joining(", "));
			out.add(new Mod(id, meta.getName(), meta.getVersion().getFriendlyString(), authors, meta.getDescription(),
				isLibrary(meta), opener(id, factories.get(id))));
		}
		// Mods with settings first, then by name; libraries last.
		out.sort(Comparator.comparing((Mod m) -> m.library())
			.thenComparing(m -> m.openConfig() == null)
			.thenComparing(m -> m.name().toLowerCase()));
		return List.copyOf(out);
	}

	private static Function<Screen, Screen> opener(String id, ConfigScreenFactory<?> factory) {
		// Cloth Config's own entry is only its demo screen.
		if (id.equals("cloth-config")) return null;
		if (factory != null) return factory::create;
		// Sodium has its own options screen but no Mod Menu entrypoint.
		if (id.equals("sodium")) return reflective("net.caffeinemc.mods.sodium.client.gui.VideoSettingsScreen", "createScreen");
		return null;
	}

	private static Function<Screen, Screen> reflective(String cls, String method) {
		try {
			var m = Class.forName(cls).getMethod(method, Screen.class);
			return parent -> {
				try {
					return (Screen) m.invoke(null, parent);
				} catch (ReflectiveOperationException e) {
					SaltConfig.LOGGER.warn("Couldn't open {}: {}", cls, e.toString());
					return null;
				}
			};
		} catch (ReflectiveOperationException e) {
			return null;
		}
	}

	private static boolean isLibrary(ModMetadata meta) {
		if (meta.getId().startsWith("fabric-") || meta.getId().equals("fabric-api")) return true;
		CustomValue mm = meta.getCustomValue("modmenu");
		if (mm == null || mm.getType() != CustomValue.CvType.OBJECT) return false;
		CustomValue badges = mm.getAsObject().get("badges");
		if (badges == null || badges.getType() != CustomValue.CvType.ARRAY) return false;
		for (CustomValue b : badges.getAsArray()) {
			if (b.getType() == CustomValue.CvType.STRING && b.getAsString().equals("library")) return true;
		}
		return false;
	}
}
