#include "MapSnowball.h"

#include "../../Constants.h"
#include "../../Graphics/GraphicsGL.h"

#include <nlnx/nx.hpp>

#include <cmath>

namespace ms
{
	namespace
	{
		constexpr float PI = 3.14159265f;
	}

	MapSnowball::MapSnowball(nl::node src)
	{
		if (src["info"]["fieldType"].get_integer() != snowball::FIELD_TYPE || !snowball::on_field())
			return;

		active = true;

		for (size_t lane = 0; lane < 2; lane++)
		{
			const snowball::Lane& rules = snowball::rules().lanes[lane];
			balls[lane] = nl::nx::map.resolve(snowball::nx_path(rules.ball))["0"];
			// Frame 0 until a hit plays both once (A 5744f7).
			snowmen[lane] = nl::nx::map.resolve(snowball::nx_path(rules.snowman));
			snowmen[lane].stop();
		}

		// Mob 0100100's (A 574df1, FUN_00989ba1(0x18704)).
		hit_sound = nl::nx::sound["Mob.img"]["0100100"]["Damage"];
	}

	MapSnowball::MapSnowball() {}

	bool MapSnowball::is_active() const
	{
		// The game is forgotten as the player leaves, while this field still
		// shows for the fade out.
		return active && snowball::on_field();
	}

	void MapSnowball::update()
	{
		if (!is_active())
			return;

		snowball::State& game = snowball::state();

		for (const snowball::Hit& hit : game.hits.take_due(snowball::now()))
			show(hit);

		if (game.state == 1)
			for (snowball::Ball& ball : game.balls)
				ball.roll(Constants::TIMESTEP, snowball::rules());

		for (Animation& snowman : snowmen)
			snowman.update();

		numbers.remove_if([](DamageNumber& number) { return number.update(); });
	}

	void MapSnowball::show(const snowball::Hit& hit)
	{
		const snowball::Rules& rules = snowball::rules();
		size_t lane = hit.what % 2;
		bool on_snowman = hit.what >= snowball::SNOWMAN_0;
		snowball::Rect rect =
				on_snowman ? rules.snowman(lane) : rules.ball(lane, snowball::state().balls[lane].step);
		Point<int16_t> at(static_cast<int16_t>((rect.left + rect.right) / 2), static_cast<int16_t>(rect.top));

		// 0 is a miss (FUN_00438a21).
		numbers.emplace_back(DamageNumber::Type::NORMAL, hit.damage);
		numbers.back().set_anchor(at);

		if (on_snowman)
		{
			// Unless it is still playing the last (A 5744f7).
			if (!snowmen[lane].is_playing())
				snowmen[lane].play(Animation::End::FIRST);

			bars[lane] = snowball::state().hp[lane];
		}

		hit_sound.play_from(at);
	}

	void MapSnowball::draw(Layer::Id layer, double viewx, double viewy, float alpha) const
	{
		if (!is_active() || layer != Layer::THREE)
			return;

		const snowball::Rules& rules = snowball::rules();
		const snowball::State& game = snowball::state();
		int64_t now = snowball::now();

		for (size_t lane = 0; lane < 2; lane++)
		{
			int32_t step = game.balls[lane].step;
			float opacity = game.fades[lane] ? snowball::fade_opacity(now - game.fades[lane]) : 1.0f;
			Point<int16_t> ball(static_cast<int16_t>(std::round(rules.ball_x(step) + viewx)),
								static_cast<int16_t>(std::round(rules.lanes[lane].y - 81 + viewy)));

			// It turns 4 degrees a step, clockwise as it rolls on (A 573ec9).
			if (opacity > 0.0f)
				balls[lane].draw(DrawArgument(step * snowball::TURN_PER_STEP * PI / 180.0f, ball, opacity));

			int16_t x = static_cast<int16_t>(std::round(rules.x + viewx));
			int16_t y = static_cast<int16_t>(std::round(rules.lanes[lane].y + viewy));
			snowmen[lane].draw(DrawArgument(Point<int16_t>(x, y)), alpha);
		}
	}

	void MapSnowball::draw_bar(size_t lane, int16_t x, int16_t y) const
	{
		// A 574606's 52x10 canvas: a black border, white, black, and the red
		// left with a dark line along its bottom.
		int16_t red = static_cast<int16_t>(snowball::hp_width(bars[lane]));
		GraphicsGL& gl = GraphicsGL::get();
		gl.drawrectangle(x, y, 52, 10, 0.0f, 0.0f, 0.0f, 1.0f);
		gl.drawrectangle(x + 1, y + 1, 50, 8, 1.0f, 1.0f, 1.0f, 1.0f);
		gl.drawrectangle(x + 2, y + 2, 48, 6, 0.0f, 0.0f, 0.0f, 1.0f);

		if (red > 0)
		{
			gl.drawrectangle(x + 3, y + 3, red, 4, 1.0f, 0.0f, 0.0f, 1.0f);
			gl.drawrectangle(x + 3, y + 6, red, 1, 0xa0 / 255.0f, 0x03 / 255.0f, 0.0f, 1.0f);
		}
	}

	void MapSnowball::draw_front(double viewx, double viewy, float alpha) const
	{
		if (!is_active())
			return;

		for (const DamageNumber& number : numbers)
			number.draw(viewx, viewy, alpha);

		const snowball::Rules& rules = snowball::rules();

		for (size_t lane = 0; lane < 2; lane++)
		{
			int16_t x = static_cast<int16_t>(std::round(rules.x - 30 + viewx));
			int16_t y = static_cast<int16_t>(std::round(rules.lanes[lane].y + viewy));
			draw_bar(lane, x, y);
		}
	}
}
