package gg.salt.menu.mixin.skin;

import com.mojang.blaze3d.pipeline.RenderPipeline;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/**
 * Every vanilla button (Button, CycleButton…) gets Salt's rounded button.
 * Before 1.21.11 there's no renderDefaultSprite, so this swaps the sprite
 * blit inside renderWidget.
 */
@Mixin(AbstractButton.class)
public abstract class AbstractButtonMixin extends AbstractWidget {
	private AbstractButtonMixin(int x, int y, int w, int h, Component message) {
		super(x, y, w, h, message);
	}

	@Redirect(method = "renderWidget", at = @At(value = "INVOKE",
		target = "Lnet/minecraft/client/gui/GuiGraphics;blitSprite(Lcom/mojang/blaze3d/pipeline/RenderPipeline;Lnet/minecraft/resources/ResourceLocation;IIIII)V"))
	private void salt$button(GuiGraphics g, RenderPipeline pipeline, ResourceLocation sprite, int x, int y, int w, int h, int color) {
		if (!Skin.on()) {
			g.blitSprite(pipeline, sprite, x, y, w, h, color);
			return;
		}
		Skin.button(g, this, x, y, w, h, active, isHoveredOrFocused(), alpha);
	}
}
