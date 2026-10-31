package gg.salt.menu.mixin.skin;

import java.util.function.Function;
import net.minecraft.client.renderer.RenderType;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.AbstractScrollArea;
import net.minecraft.resources.ResourceLocation;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Scrollbars: a thin pill thumb on a faint pill track. */
@Mixin(AbstractScrollArea.class)
public abstract class ScrollAreaMixin {
	@Redirect(method = "renderScrollbar", at = @At(value = "INVOKE",
		target = "Lnet/minecraft/client/gui/GuiGraphics;blitSprite(Ljava/util/function/Function;Lnet/minecraft/resources/ResourceLocation;IIII)V"))
	private void salt$scrollbar(GuiGraphics g, Function<ResourceLocation, RenderType> pipeline, ResourceLocation sprite, int x, int y, int w, int h) {
		if (!Skin.on()) {
			g.blitSprite(pipeline, sprite, x, y, w, h);
			return;
		}
		Skin.scrollbar(g, sprite.getPath().contains("background"), x, y, w, h);
	}
}
