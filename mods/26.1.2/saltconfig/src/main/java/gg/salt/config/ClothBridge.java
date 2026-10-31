package gg.salt.config;

import com.google.common.collect.ImmutableList;
import me.shedaniel.clothconfig2.api.AbstractConfigEntry;
import me.shedaniel.clothconfig2.gui.AbstractConfigScreen;
import me.shedaniel.clothconfig2.gui.entries.AbstractNumberListEntry;
import me.shedaniel.clothconfig2.gui.entries.BooleanListEntry;
import me.shedaniel.clothconfig2.gui.entries.DoubleListEntry;
import me.shedaniel.clothconfig2.gui.entries.FloatListEntry;
import me.shedaniel.clothconfig2.gui.entries.IntegerListEntry;
import me.shedaniel.clothconfig2.gui.entries.IntegerSliderEntry;
import me.shedaniel.clothconfig2.gui.entries.LongListEntry;
import me.shedaniel.clothconfig2.gui.entries.LongSliderEntry;
import me.shedaniel.clothconfig2.gui.entries.SelectionListEntry;
import me.shedaniel.clothconfig2.gui.entries.SubCategoryListEntry;
import me.shedaniel.clothconfig2.gui.entries.TextFieldListEntry;
import me.shedaniel.clothconfig2.gui.entries.TextListEntry;
import me.shedaniel.clothconfig2.gui.entries.TooltipListEntry;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Function;

/**
 * Turns a Cloth Config screen into saltConfig rows, so other mods' settings
 * show inside saltConfig instead of in their own screen. The screen is built
 * but never opened: its entries hold the values, and {@code saveAll(false)}
 * runs the mod's own save code. Only touched when Cloth Config is installed
 * (SaltConfigScreen checks first), since it links against Cloth's classes.
 */
final class ClothBridge {
	/** Rows for one mod plus how to save them. */
	record Page(List<ExtRow> rows, Runnable save, boolean[] dirty) {}

	private ClothBridge() {}

	/** Null when the screen isn't a Cloth Config screen. */
	static Page read(Screen screen) {
		if (!(screen instanceof AbstractConfigScreen cloth)) return null;
		List<ExtRow> rows = new ArrayList<>();
		boolean[] dirty = {false};
		Map<Component, List<AbstractConfigEntry<?>>> cats = cloth.getCategorizedEntries();
		for (var cat : cats.entrySet()) {
			if (cats.size() > 1) rows.add(ExtRow.header(cat.getKey().getString()));
			for (AbstractConfigEntry<?> e : cat.getValue()) add(rows, e, dirty);
		}
		return new Page(rows, () -> cloth.saveAll(false), dirty);
	}

	@SuppressWarnings({"unchecked", "rawtypes"})
	private static void add(List<ExtRow> rows, Object entry, boolean[] dirty) {
		try {
			String label = entry instanceof AbstractConfigEntry<?> ace ? pretty(ace.getFieldName().getString()) : "";
			String desc = entry instanceof TooltipListEntry<?> t ? t.getTooltip().map(ClothBridge::firstLine).orElse("") : "";
			switch (entry) {
				case SubCategoryListEntry sub -> {
					rows.add(ExtRow.header(sub.getCategoryName().getString()));
					for (Object child : sub.getValue()) add(rows, child, dirty);
				}
				case TextListEntry text -> {
					Component c = (Component) get(text, "text");
					if (c != null && !c.getString().isBlank()) rows.add(ExtRow.text(c.getString()));
				}
				case BooleanListEntry b -> {
					AtomicBoolean v = (AtomicBoolean) get(b, "bool");
					rows.add(ExtRow.toggle(label, desc, v::get, on -> { v.set(on); dirty[0] = true; }));
				}
				case SelectionListEntry s -> {
					ImmutableList<Object> values = (ImmutableList<Object>) get(s, "values");
					AtomicInteger index = (AtomicInteger) get(s, "index");
					Function<Object, Component> names = (Function<Object, Component>) get(s, "nameProvider");
					rows.add(ExtRow.choice(label, desc,
						() -> name(names, values.get(index.get())),
						() -> { index.set(Math.floorMod(index.get() + 1, values.size())); dirty[0] = true; },
						() -> { index.set(Math.floorMod(index.get() - 1, values.size())); dirty[0] = true; }));
				}
				case IntegerSliderEntry s -> {
					int min = (int) get(s, "minimum"), max = (int) get(s, "maximum");
					rows.add(ExtRow.slider(label, desc,
						() -> s.getTextGetter().apply(s.getValue()).getString(),
						() -> max == min ? 0 : (s.getValue() - min) / (double) (max - min),
						f -> { s.setValue((int) Math.round(min + f * (max - min))); dirty[0] = true; }));
				}
				case LongSliderEntry s -> {
					long min = (long) get(s, "minimum"), max = (long) get(s, "maximum");
					rows.add(ExtRow.slider(label, desc,
						() -> s.getTextGetter().apply(s.getValue()).getString(),
						() -> max == min ? 0 : (s.getValue() - min) / (double) (max - min),
						f -> { s.setValue(Math.round(min + f * (max - min))); dirty[0] = true; }));
				}
				case IntegerListEntry n -> rows.add(stepper(label, desc, n, 1, dirty));
				case LongListEntry n -> rows.add(stepper(label, desc, n, 1, dirty));
				case FloatListEntry n -> rows.add(stepper(label, desc, n, 0.1, dirty));
				case DoubleListEntry n -> rows.add(stepper(label, desc, n, 0.1, dirty));
				case TextFieldListEntry t -> {
					EditBox box = (EditBox) get(t, "textFieldWidget");
					rows.add(ExtRow.readonly(label, desc, box::getValue));
				}
				case AbstractConfigEntry<?> other -> rows.add(ExtRow.readonly(label, desc, () -> String.valueOf(other.getValue())));
				default -> {}
			}
		} catch (Throwable t) {
			SaltConfig.LOGGER.debug("Skipped a Cloth entry: {}", t.toString());
		}
	}

	/** Number text field: click steps up, right-click steps down, clamped to its range. */
	private static ExtRow stepper(String label, String desc, AbstractNumberListEntry<?> n, double step, boolean[] dirty) {
		EditBox box = (EditBox) get(n, "textFieldWidget");
		Number min = (Number) get(n, "minimum"), max = (Number) get(n, "maximum");
		boolean whole = step >= 1;
		Runnable[] nudge = new Runnable[2];
		for (int i = 0; i < 2; i++) {
			double dir = i == 0 ? 1 : -1;
			nudge[i] = () -> {
				double v;
				try {
					v = Double.parseDouble(box.getValue().trim());
				} catch (NumberFormatException e) {
					v = 0;
				}
				v += dir * step;
				if (min != null) v = Math.max(min.doubleValue(), v);
				if (max != null) v = Math.min(max.doubleValue(), v);
				n.setValue(whole ? String.valueOf(Math.round(v)) : String.format(Locale.ROOT, "%.2f", v).replaceAll("0$", ""));
				dirty[0] = true;
			};
		}
		return ExtRow.stepper(label, desc, box::getValue, nudge[0], nudge[1]);
	}

	private static String name(Function<Object, Component> names, Object value) {
		if (names != null) {
			Component c = names.apply(value);
			if (c != null) return c.getString();
		}
		return String.valueOf(value);
	}

	/** Untranslated keys ("mod.config.option.rainCulling") become "Rain culling". */
	private static String pretty(String label) {
		if (label.contains(" ") || !label.contains(".")) return label;
		String last = label.substring(label.lastIndexOf('.') + 1).replace('_', ' ');
		String words = last.replaceAll("([a-z0-9])([A-Z])", "$1 $2").toLowerCase(Locale.ROOT);
		return words.isEmpty() ? label : Character.toUpperCase(words.charAt(0)) + words.substring(1);
	}

	private static String firstLine(Component[] lines) {
		return lines.length == 0 ? "" : lines[0].getString();
	}

	/** Reads a (usually private) field, searching up the class hierarchy. */
	private static Object get(Object target, String name) {
		for (Class<?> c = target.getClass(); c != null; c = c.getSuperclass()) {
			try {
				Field f = c.getDeclaredField(name);
				f.setAccessible(true);
				return f.get(target);
			} catch (NoSuchFieldException ignored) {
				// keep looking in the superclass
			} catch (IllegalAccessException e) {
				throw new IllegalStateException(e);
			}
		}
		throw new IllegalStateException("no field " + name + " on " + target.getClass().getName());
	}
}
