#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Character/FriendGroups.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <array>
#include <string>
#include <vector>

namespace ms
{
	// v83's Edit Group dialog (UserList/Friend/GroupMod, FUN_0080ee78): a
	// group's name over every buddy, each with a checkbox. OK moves the
	// buddies checked into the group and the others in it out, one add each,
	// since the server knows a group only as the name on each buddy.
	class UIFriendGroup : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::FRIENDGROUP;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// group is blank for a new one, which starts with nobody checked.
		UIFriendGroup(const std::string& group, const std::vector<Buddy>& buddies);

		void draw(float alpha) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void scroll(int16_t row);
		void ok();
		void close();

		enum Buttons : uint16_t
		{
			CLOSE,
			OK,
			CANCEL
		};

		static constexpr size_t SHOWN = 7;

		std::string group;
		bool renaming;
		std::vector<FriendGroups::Pick> picks;
		std::vector<std::string> taken;
		Texture background;
		std::array<Texture, 2> checkbox;
		Textfield name;
		std::vector<Text> names, groups;
		Slider slider;
		int16_t top = 0;
	};
}
