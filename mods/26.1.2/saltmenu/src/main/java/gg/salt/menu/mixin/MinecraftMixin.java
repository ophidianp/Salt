package gg.salt.menu.mixin;

import gg.salt.menu.SaltMenu;
import gg.salt.menu.SaltTitleScreen;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyVariable;

/** 26.1 sets screens through Minecraft#setScreen; swap the vanilla title screen there. */
@Mixin(Minecraft.class)
public class MinecraftMixin {
	@ModifyVariable(method = "setScreen", at = @At("HEAD"), argsOnly = true)
	private Screen saltmenu$replaceTitle(Screen screen) {
		if (!SaltMenu.CFG.enabled()) return screen;
		// setScreen(null) with no world makes vanilla build a TitleScreen
		// *inside* setScreen (that's how the first menu appears on 26.1),
		// past this hook, so create it here instead.
		if (screen == null && ((Minecraft) (Object) this).level == null && !SaltMenu.showVanillaOnce) {
			screen = new TitleScreen();
		}
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
