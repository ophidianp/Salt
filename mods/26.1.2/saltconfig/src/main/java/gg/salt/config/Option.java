package gg.salt.config;

import java.util.List;

/** The kinds of settings a saltConfig page can show. */
public sealed interface Option {
	String label();

	String description();

	record Toggle(String key, String label, String description, boolean def) implements Option {}

	record Slider(String key, String label, String description, double min, double max, double step, double def) implements Option {}

	record Choice(String key, String label, String description, List<String> choices, String def) implements Option {}

	record Action(String label, String description, Runnable run) implements Option {}
}
