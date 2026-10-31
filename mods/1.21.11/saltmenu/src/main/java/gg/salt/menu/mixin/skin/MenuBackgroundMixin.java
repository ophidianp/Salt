package gg.salt.menu.mixin.skin;

import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Menu backgrounds: a flat dark tint over the blurred panorama (or world) instead of vanilla's texture. */
@Mixin(Screen.class)
public abstract class MenuBackgroundMixin {
	@Inject(method = "renderMenuBackgroundTexture", at = @At("HEAD"), cancellable = true)
	private static void salt$menuBackground(GuiGraphics g, Identifier texture, int x, int y, float u, float v, int w, int h, CallbackInfo ci) {
		if (!Skin.on()) return;
		// In a world the game stays visible behind menus, like vanilla.
		boolean inWorld = texture.getPath().contains("inworld");
		g.fill(x, y, x + w, y + h, inWorld ? 0x66000000 : 0xA0050608);
		ci.cancel();
	}
}
