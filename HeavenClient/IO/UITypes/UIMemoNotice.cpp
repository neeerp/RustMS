#include "UIMemoNotice.h"
#include "UIMemo.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <limits>

namespace ms
{
	namespace
	{
		// At (463, 498) on v83's 800x600 screen, whatever else is showing.
		constexpr int16_t LEFT = 463, ABOVE_BOTTOM = 102;
		const Point<int16_t> OK_AT(136, 7), CANCEL_AT(136, 20);
	}

	UIMemoNotice::UIMemoNotice() : UIElement(Point<int16_t>(), Point<int16_t>()),
		life(std::numeric_limits<int32_t>::max())
	{
		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		// Its words are part of the picture.
		background = fade["backgrnd3"];
		dimension = background.get_dimensions();

		buttons[OK] = std::make_unique<MapleButton>(fade["BtOK"], OK_AT);
		buttons[CANCEL] = std::make_unique<MapleButton>(fade["BtCancel"], CANCEL_AT);

		Sound(Sound::Name::INVITE).play();

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIMemoNotice::draw(float alpha) const
	{
		float opacity = life.opacity();

		background.draw(DrawArgument(position, opacity));

		for (auto& [id, button] : buttons)
			static_cast<MapleButton*>(button.get())->set_opacity(opacity);

		UIElement::draw_buttons(alpha);
	}

	void UIMemoNotice::update()
	{
		life.update(Constants::TIMESTEP);

		if (life.gone())
			deactivate();

		UIElement::update();
	}

	void UIMemoNotice::update_screen(int16_t, int16_t new_height)
	{
		position = Point<int16_t>(LEFT, new_height - ABOVE_BOTTOM);
	}

	bool UIMemoNotice::showing()
	{
		auto notice = UI::get().get_element<UIMemoNotice>();

		return notice && notice->is_active() && !notice->life.closed();
	}

	Button::State UIMemoNotice::button_pressed(uint16_t buttonid)
	{
		if (life.closed())
			return Button::State::NORMAL;

		life.close();

		if (buttonid == OK)
			UIMemo::open();

		return Button::State::NORMAL;
	}

	UIElement::Type UIMemoNotice::get_type() const
	{
		return TYPE;
	}
}
