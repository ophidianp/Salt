package gg.salt.menu.mixin.skin;

import com.mojang.blaze3d.pipeline.RenderPipeline;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractScrollArea;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Scrollbars: a thin pill thumb on a faint pill track. */
@Mixin(AbstractScrollArea.class)
public abstract class ScrollAreaMixin {
	@Redirect(method = "extractScrollbar", at = @At(value = "INVOKE",
		target = "Lnet/minecraft/client/gui/GuiGraphicsExtractor;blitSprite(Lcom/mojang/blaze3d/pipeline/RenderPipeline;Lnet/minecraft/resources/Identifier;IIII)V"))
	private void salt$scrollbar(GuiGraphicsExtractor g, RenderPipeline pipeline, Identifier sprite, int x, int y, int w, int h) {
		if (!Skin.on()) {
			g.blitSprite(pipeline, sprite, x, y, w, h);
			return;
		}
		Skin.scrollbar(g, sprite.getPath().contains("background"), x, y, w, h);
	}
}
