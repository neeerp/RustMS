#include "UIInvitation.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Character/Marriage.h"
#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// v83 draws its text about 6 pixels higher than ours.
		constexpr int16_t TEXT_RAISE = 6;
		// Left-aligned, unclipped (0x7ecca1, 0x7ecd77).
		const Point<int16_t> GROOM_AT = Point<int16_t>(50, 105 - TEXT_RAISE);
		const Point<int16_t> BRIDE_AT = Point<int16_t>(131, 105 - TEXT_RAISE);
		const Point<int16_t> OK_AT = Point<int16_t>(87, 206);
	}

	UIInvitation::UIInvitation(const std::string& groom_name, const std::string& bride_name, int16_t type) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0))
	{
		nl::node invitation = NxFiles::ui_classic["UIWindow.img"]["Wedding"]["Invitation"];

		background = invitation[marriage::invitation_art(type)];
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2, (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(invitation["BtOK"], OK_AT);

		groom = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, groom_name);
		bride = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, bride_name);
	}

	void UIInvitation::draw(float inter) const
	{
		background.draw(position);
		groom.draw(position + GROOM_AT);
		bride.draw(position + BRIDE_AT);

		UIElement::draw(inter);
	}

	void UIInvitation::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			deactivate();
	}

	UIElement::Type UIInvitation::get_type() const
	{
		return TYPE;
	}

	Button::State UIInvitation::button_pressed(uint16_t buttonid)
	{
		deactivate();

		return Button::State::NORMAL;
	}
}
