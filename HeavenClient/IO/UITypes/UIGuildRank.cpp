#include "UIGuildRank.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Character/GuildRankRules.h"
#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// FUN_005958c0 and FUN_00595d86, in the window.
		constexpr Point<int16_t> OK_BUTTON = Point<int16_t>(148, 297);
		constexpr Point<int16_t> LEFT_BUTTON = Point<int16_t>(86, 297);
		constexpr Point<int16_t> RIGHT_BUTTON = Point<int16_t>(217, 297);
		constexpr Point<int16_t> CROWN = Point<int16_t>(155, 43);
		constexpr Point<int16_t> SPARKLE = Point<int16_t>(127, 23);
		constexpr int16_t LEADER_Y = 80;
		constexpr int16_t FIRST_ROW_Y = 137;
		constexpr int16_t ROW_PITCH = 25;
		constexpr int16_t RANK_X = 44;
		constexpr int16_t EMBLEM_X = 65;
		constexpr int16_t EMBLEM_RAISE = 4;
		constexpr int16_t NAME_X = 85;
		constexpr int16_t POINTS_RIGHT = 275;
		// v83 draws a line's cap top about 3 below where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
	}

	UIGuildRank::UIGuildRank(std::vector<GuildStanding> guilds, const GuildEmblem &own)
			: UIElement(Point<int16_t>(), Point<int16_t>()), page(1)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["UserList"]["Guild"]["GuildRank"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);
		sprites.emplace_back(src["icon0"], DrawArgument(CROWN));
		sprites.emplace_back(src["IconEff"], DrawArgument(SPARKLE));

		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOK"], OK_BUTTON);
		buttons[Buttons::LEFT] = std::make_unique<MapleButton>(src["BtLeft"], LEFT_BUTTON);
		buttons[Buttons::RIGHT] = std::make_unique<MapleButton>(src["BtRight"], RIGHT_BUTTON);

		for (size_t i = 0; i < guilds.size(); i++)
		{
			const GuildStanding &guild = guilds[i];
			int32_t rank = static_cast<int32_t>(i) + 1;

			rows.push_back({
				Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, guild_rank::rank_text(rank)),
				guild_rank::has_emblem(guild.emblem) ? GuildMark(guild_rank::shown_emblem(guild.emblem, own))
													 : GuildMark(),
				Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, guild.name),
				Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK, std::to_string(guild.points))
			});
		}

		// Centred on the screen, with no title bar to drag it by.
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		turn_to(1);
	}

	void UIGuildRank::draw(float alpha) const
	{
		draw_sprites(alpha);

		if (!rows.empty())
			draw_row(1, LEADER_Y);

		int32_t first = guild_rank::first_rank(page);

		for (int32_t i = 0; i < guild_rank::PAGE_ROWS; i++)
			draw_row(first + i, static_cast<int16_t>(FIRST_ROW_Y + ROW_PITCH * i));

		UIElement::draw_buttons(alpha);
	}

	void UIGuildRank::draw_row(int32_t rank, int16_t y) const
	{
		if (rank < 1 || rank > static_cast<int32_t>(rows.size()))
			return;

		const Row &row = rows[rank - 1];
		float opacity = guild_rank::alpha(rank) / 255.0f;
		Point<int16_t> raise = Point<int16_t>(0, TEXT_RAISE);

		row.rank.draw(DrawArgument(position + Point<int16_t>(RANK_X, y) - raise, opacity));
		row.emblem.draw(position + Point<int16_t>(EMBLEM_X, y - EMBLEM_RAISE));
		row.name.draw(DrawArgument(position + Point<int16_t>(NAME_X, y) - raise, opacity));
		row.points.draw(DrawArgument(position + Point<int16_t>(POINTS_RIGHT, y) - raise, opacity));
	}

	void UIGuildRank::turn_to(int32_t to)
	{
		int32_t count = static_cast<int32_t>(rows.size());
		page = to;

		buttons[Buttons::LEFT]->set_state(guild_rank::can_go_left(page) ? Button::State::NORMAL
																		 : Button::State::DISABLED);
		buttons[Buttons::RIGHT]->set_state(guild_rank::can_go_right(page, count) ? Button::State::NORMAL
																				 : Button::State::DISABLED);
	}

	void UIGuildRank::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// FUN_004edd21: Enter presses OK, and Escape closes.
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			deactivate();
	}

	UIElement::Type UIGuildRank::get_type() const
	{
		return TYPE;
	}

	Button::State UIGuildRank::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::OK:
			deactivate();
			break;
		case Buttons::LEFT:
			turn_to(page - 1);
			// turn_to set the button's state, which a press must not undo.
			return buttons[Buttons::LEFT]->get_state();
		case Buttons::RIGHT:
			turn_to(page + 1);
			return buttons[Buttons::RIGHT]->get_state();
		default:
			break;
		}

		return Button::State::NORMAL;
	}
}
