package gg.salt.config;

/** Before 1.21.9, screens get mouse input as (x, y, button); this carries it like 1.21.9+'s MouseButtonEvent. */
public record MouseEvent(double x, double y, int button) {}
