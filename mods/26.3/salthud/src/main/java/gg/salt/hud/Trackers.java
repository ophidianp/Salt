package gg.salt.hud;

import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.Minecraft;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.phys.Vec3;

import java.util.ArrayDeque;

/** Live stats the HUD shows that vanilla doesn't track: CPS, speed, combo, reach. */
public final class Trackers {
	private Trackers() {}

	private static final ArrayDeque<Long> LEFT = new ArrayDeque<>();
	private static final ArrayDeque<Long> RIGHT = new ArrayDeque<>();

	/** Called from the mouse mixin on each button press. */
	public static void click(int button) {
		long now = System.currentTimeMillis();
		if (button == InputConstants.MOUSE_BUTTON_LEFT) LEFT.addLast(now);
		else if (button == InputConstants.MOUSE_BUTTON_RIGHT) RIGHT.addLast(now);
	}

	private static int count(ArrayDeque<Long> q) {
		long cutoff = System.currentTimeMillis() - 1000;
		while (!q.isEmpty() && q.peekFirst() < cutoff) q.pollFirst();
		return q.size();
	}

	public static int leftCps() { return count(LEFT); }
	public static int rightCps() { return count(RIGHT); }

	// ---- speed (blocks per second, horizontal), sampled each client tick
	private static Vec3 lastPos;
	private static double speed;

	// ---- combo + reach
	private static int combo;
	private static long lastHit;
	private static int lastHurtTime;
	private static double reach;
	private static long reachTime;

	public static void tick(Minecraft mc) {
		if (mc.player == null) {
			lastPos = null;
			return;
		}
		Vec3 pos = mc.player.position();
		if (lastPos != null) {
			double dx = pos.x - lastPos.x, dz = pos.z - lastPos.z;
			speed = speed * 0.5 + Math.sqrt(dx * dx + dz * dz) * 20.0 * 0.5;
		}
		lastPos = pos;
		// Taking damage, or 3 s without a hit, ends a combo.
		int hurt = mc.player.hurtTime;
		if (hurt > lastHurtTime) combo = 0;
		lastHurtTime = hurt;
		if (System.currentTimeMillis() - lastHit > 3000) combo = 0;
	}

	public static void attack(Entity target) {
		Minecraft mc = Minecraft.getInstance();
		if (mc.player == null) return;
		combo++;
		lastHit = System.currentTimeMillis();
		Vec3 eye = mc.player.getEyePosition();
		reach = Math.sqrt(target.getBoundingBox().distanceToSqr(eye));
		reachTime = lastHit;
	}

	public static double speed() { return speed; }
	public static int combo() { return combo; }

	/** Last hit distance, or -1 if older than 3 s. */
	public static double reach() {
		return System.currentTimeMillis() - reachTime < 3000 ? reach : -1;
	}
}
