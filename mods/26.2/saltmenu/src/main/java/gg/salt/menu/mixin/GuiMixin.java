package gg.salt.menu.mixin;

import gg.salt.menu.SaltMenu;
import gg.salt.menu.SaltTitleScreen;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Gui;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyVariable;

/** 26.3 sets screens through Gui#setScreen; swap the vanilla title screen there. */
@Mixin(Gui.class)
public class GuiMixin {
	@ModifyVariable(method = "setScreen", at = @At("HEAD"), argsOnly = true)
	private Screen saltmenu$replaceTitle(Screen screen) {
		if (!SaltMenu.CFG.enabled()) return screen;
		// Closing to "no screen" outside a world (Create World's Cancel, the
		// HUD editor...) makes setScreen build a vanilla TitleScreen itself.
		if (screen == null && Minecraft.getInstance().level == null) return new SaltTitleScreen();
		if (screen instanceof TitleScreen && !(screen instanceof SaltTitleScreen)) {
			if (SaltMenu.showVanillaOnce) {
				SaltMenu.showVanillaOnce = false;
				return screen;
			}
			if (!SaltMenu.loggedOnce) {
				SaltMenu.loggedOnce = true;
				SaltMenu.LOGGER.info("saltMenu: showing the Salt title screen");
			}
			return new SaltTitleScreen();
		}
		return screen;
	}
}
