package gg.salt.menu.mixin.skin;

import com.mojang.renderpearl.api.pipeline.RenderPipeline;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractSliderButton;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Sliders: rounded track with an accent fill, and a rounded handle. */
@Mixin(AbstractSliderButton.class)
public abstract class SliderMixin extends AbstractWidget {
	private static final String BLIT = "Lnet/minecraft/client/gui/GuiGraphicsExtractor;blitSprite(Lcom/mojang/renderpearl/api/pipeline/RenderPipeline;Lnet/minecraft/resources/Identifier;IIIII)V";

	@Shadow
	protected double value;

	private SliderMixin(int x, int y, int w, int h, Component message) {
		super(x, y, w, h, message);
	}

	@Redirect(method = "extractWidgetRenderState", at = @At(value = "INVOKE", target = BLIT, ordinal = 0))
	private void salt$track(GuiGraphicsExtractor g, RenderPipeline pipeline, Identifier sprite, int x, int y, int w, int h, int color) {
		if (!Skin.on()) {
			g.blitSprite(pipeline, sprite, x, y, w, h, color);
			return;
		}
		Skin.sliderTrack(g, this, x, y, w, h, value, active, isHoveredOrFocused(), alpha);
	}

	@Redirect(method = "extractWidgetRenderState", at = @At(value = "INVOKE", target = BLIT, ordinal = 1))
	private void salt$handle(GuiGraphicsExtractor g, RenderPipeline pipeline, Identifier sprite, int x, int y, int w, int h, int color) {
		if (!Skin.on()) {
			g.blitSprite(pipeline, sprite, x, y, w, h, color);
			return;
		}
		Skin.sliderHandle(g, x, y, w, h, active, isHoveredOrFocused(), alpha);
	}
}
