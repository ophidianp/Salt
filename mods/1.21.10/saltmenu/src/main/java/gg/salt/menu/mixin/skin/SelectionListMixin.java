package gg.salt.menu.mixin.skin;

import gg.salt.config.SaltUi;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.AbstractSelectionList;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.layouts.LayoutElement;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Coerce;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Lists (worlds, servers, options…): a soft dark panel, hairline edges, rounded selection. */
@Mixin(AbstractSelectionList.class)
public abstract class SelectionListMixin extends AbstractWidget {
	private SelectionListMixin(int x, int y, int w, int h, Component message) {
		super(x, y, w, h, message);
	}

	@Inject(method = "renderListBackground", at = @At("HEAD"), cancellable = true)
	private void salt$background(GuiGraphics g, CallbackInfo ci) {
		if (!Skin.on()) return;
		g.fill(getX(), getY(), getRight(), getBottom(), Skin.LIST_BG);
		ci.cancel();
	}

	@Inject(method = "renderListSeparators", at = @At("HEAD"), cancellable = true)
	private void salt$separators(GuiGraphics g, CallbackInfo ci) {
		if (!Skin.on()) return;
		g.fill(getX(), getY() - 1, getRight(), getY(), SaltUi.BORDER);
		g.fill(getX(), getBottom(), getRight(), getBottom() + 1, SaltUi.BORDER);
		ci.cancel();
	}

	@Inject(method = "renderSelection", at = @At("HEAD"), cancellable = true)
	private void salt$selection(GuiGraphics g, @Coerce LayoutElement entry, int color, CallbackInfo ci) {
		if (!Skin.on()) return;
		Skin.selection(g, entry.getX(), entry.getY(), entry.getWidth(), entry.getHeight(), isFocused());
		ci.cancel();
	}
}
