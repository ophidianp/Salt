package gg.salt.menu.mixin.skin;

import com.mojang.blaze3d.pipeline.RenderPipeline;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.TabButton;
import net.minecraft.client.gui.components.tabs.MenuTabBar;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Tabs (Create World, Options…): rounded, accent-tinted when selected. */
@Mixin(MenuTabBar.MenuTabButton.class)
public abstract class MenuTabButtonMixin {
	@Redirect(method = "extractWidgetRenderState", at = @At(value = "INVOKE",
		target = "Lnet/minecraft/client/gui/GuiGraphicsExtractor;blitSprite(Lcom/mojang/blaze3d/pipeline/RenderPipeline;Lnet/minecraft/resources/Identifier;IIII)V"))
	private void salt$tab(GuiGraphicsExtractor g, RenderPipeline pipeline, Identifier sprite, int x, int y, int w, int h) {
		if (!Skin.on()) {
			g.blitSprite(pipeline, sprite, x, y, w, h);
			return;
		}
		TabButton self = (TabButton) (Object) this;
		Skin.tab(g, this, x, y, w, h, self.isSelected(), self.isHoveredOrFocused());
	}
}
