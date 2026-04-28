use gpui::{Context, Entity};

pub fn setter<T: 'static, V: 'static, F: Fn(&mut V, T) + Clone>(entity: Entity<V>, f: F) -> impl Fn(T, &mut Context<V>) + Clone {
	move |value, cx| {
		entity.update(cx, |this, cx| {
			f(this, value);
			cx.notify();
		});
	}
}
