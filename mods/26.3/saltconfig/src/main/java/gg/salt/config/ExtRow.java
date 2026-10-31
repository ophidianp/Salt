package gg.salt.config;

import java.util.function.BooleanSupplier;
import java.util.function.Consumer;
import java.util.function.DoubleConsumer;
import java.util.function.DoubleSupplier;
import java.util.function.Supplier;

/**
 * One row of another mod's settings, shown inline in saltConfig. Built by
 * {@link ClothBridge} from a Cloth Config entry; the lambdas read and write
 * that entry, so saving goes through the mod's own save code.
 */
public record ExtRow(Kind kind, String label, String description, BooleanSupplier bool, Consumer<Boolean> setBool,
		Supplier<String> value, Runnable next, Runnable prev, DoubleSupplier frac, DoubleConsumer setFrac) {
	public enum Kind { HEADER, TEXT, TOGGLE, CHOICE, SLIDER, STEPPER, READONLY }

	public static ExtRow header(String label) {
		return new ExtRow(Kind.HEADER, label, "", null, null, null, null, null, null, null);
	}

	public static ExtRow text(String text) {
		return new ExtRow(Kind.TEXT, text, "", null, null, null, null, null, null, null);
	}

	public static ExtRow toggle(String label, String desc, BooleanSupplier get, Consumer<Boolean> set) {
		return new ExtRow(Kind.TOGGLE, label, desc, get, set, null, null, null, null, null);
	}

	public static ExtRow choice(String label, String desc, Supplier<String> value, Runnable next, Runnable prev) {
		return new ExtRow(Kind.CHOICE, label, desc, null, null, value, next, prev, null, null);
	}

	public static ExtRow slider(String label, String desc, Supplier<String> value, DoubleSupplier frac, DoubleConsumer setFrac) {
		return new ExtRow(Kind.SLIDER, label, desc, null, null, value, null, null, frac, setFrac);
	}

	/** A number field: shown as ‹ value › with click = +1 step, right-click = −1. */
	public static ExtRow stepper(String label, String desc, Supplier<String> value, Runnable inc, Runnable dec) {
		return new ExtRow(Kind.STEPPER, label, desc, null, null, value, inc, dec, null, null);
	}

	public static ExtRow readonly(String label, String desc, Supplier<String> value) {
		return new ExtRow(Kind.READONLY, label, desc, null, null, value, null, null, null, null);
	}

	public int height() {
		return switch (kind) {
			case HEADER -> 18;
			case TEXT -> 12;
			default -> 24;
		};
	}
}
