#include "UIAvatarMegaphone.h"

#include "../../Constants.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		constexpr int16_t WIDTH = 225;
		constexpr int16_t HEIGHT = 121;
		// From the banner's top left (FUN_0045a825, FUN_0045aef1): where the speaker stands, the name plate's top,
		// the name's centre, and the lines' left and tops. v83 draws a line's cap top about 3 below where it is
		// drawn, we 9, so text goes up by the difference.
		constexpr Point<int16_t> AVATAR = Point<int16_t>(82, 85);
		constexpr int16_t PLATE_Y = 84;
		constexpr int16_t SHORT_PLATE_X = 48;
		constexpr int16_t LONG_PLATE_X = 3;
		constexpr int16_t NAME_X = 78;
		constexpr int16_t LINES_X = 124;
		constexpr int16_t LINES_Y = 14;
		constexpr int16_t LINE_HEIGHT = 18;
		constexpr int16_t TEXT_RAISE = 6;
		// The slide in and out takes 225 ms; then it wobbles 5 px six times, 45 ms each (FUN_0045b2b9).
		constexpr int32_t SLIDE = 225;
		constexpr int32_t WOBBLE = 45;
		constexpr int32_t WOBBLES = 6;
		constexpr int16_t WOBBLE_X = 5;
		// It slides out after 10 s (CWvsContext::Update).
		constexpr int32_t SHOWN_FOR = 10'000;

		// The item's `info/path`, as "Map/MapHelper.img/AvatarMegaphone/Burning".
		nl::node art_of(int32_t item_id)
		{
			nl::node info = nl::nx::item["Cash"]["0539.img"][string_format::extend_id(item_id, 8)]["info"];
			std::string path = info["path"];
			std::string file = path.substr(0, path.find('/'));

			if (file != "Map" || file.size() == path.size())
				return nl::node();

			nl::node node = nl::nx::map;

			for (size_t first = file.size() + 1; first <= path.size();)
			{
				size_t last = std::min(path.find('/', first), path.size());
				node = node[path.substr(first, last - first)];
				first = last + 1;
			}

			return node;
		}
	}

	UIAvatarMegaphone::UIAvatarMegaphone(int32_t item_id, const std::string &speaker,
										 const Megaphones::AvatarLines &said, const LookEntry &entry)
			: UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT)), art(art_of(item_id)), look(entry),
			  shown(0), leaving(-1), left_at(0)
	{
		int32_t emotion = nl::nx::item["Cash"]["0539.img"][string_format::extend_id(item_id, 8)]["info"]["emotion"];
		expression = Expression::byaction(emotion + 99);

		name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::WHITE, speaker);

		bool wide = Megaphones::avatar_long_name(name.width());
		plate = nl::nx::map["MapHelper.img"]["AvatarMegaphone"]["name"][wide ? "1" : "0"];
		plate_x = wide ? LONG_PLATE_X : SHORT_PLATE_X;

		for (size_t i = 0; i < lines.size(); i++)
			lines[i] = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, said[i]);

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIAvatarMegaphone::draw(float alpha) const
	{
		Point<int16_t> at = position + Point<int16_t>(offset(), 0);

		art.draw(at, alpha);
		look.draw(at + AVATAR, false, Stance::Id::STAND1, expression);
		plate.draw(at + Point<int16_t>(plate_x, PLATE_Y));
		name.draw(at + Point<int16_t>(NAME_X, PLATE_Y - TEXT_RAISE));

		for (size_t i = 0; i < lines.size(); i++)
			lines[i].draw(at + Point<int16_t>(LINES_X, LINES_Y + LINE_HEIGHT * static_cast<int16_t>(i) - TEXT_RAISE));
	}

	void UIAvatarMegaphone::update()
	{
		UIElement::update();

		art.update();
		shown += Constants::TIMESTEP;

		if (leaving < 0 && shown >= SHOWN_FOR)
			hide();

		if (leaving >= 0)
		{
			leaving += Constants::TIMESTEP;

			if (leaving >= SLIDE)
				deactivate();
		}
	}

	void UIAvatarMegaphone::update_screen(int16_t new_width, int16_t)
	{
		position = Point<int16_t>(new_width - WIDTH, 0);
	}

	bool UIAvatarMegaphone::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIAvatarMegaphone::get_type() const
	{
		return TYPE;
	}

	void UIAvatarMegaphone::hide()
	{
		if (leaving < 0)
		{
			left_at = offset();
			leaving = 0;
		}
	}

	int16_t UIAvatarMegaphone::offset() const
	{
		// Out from wherever it was when told to leave, so a clear during the slide in doesn't jump.
		if (leaving >= 0)
			return static_cast<int16_t>(left_at + (WIDTH - left_at) * std::min(leaving, SLIDE) / SLIDE);

		if (shown < SLIDE)
			return static_cast<int16_t>(WIDTH * (SLIDE - shown) / SLIDE);

		int32_t wobble = (shown - SLIDE) / WOBBLE;

		if (wobble >= WOBBLES)
			return 0;

		int32_t into = (shown - SLIDE) % WOBBLE;

		// Out 5 px and back, three times.
		return static_cast<int16_t>(WOBBLE_X * (wobble % 2 ? WOBBLE - into : into) / WOBBLE);
	}
}
