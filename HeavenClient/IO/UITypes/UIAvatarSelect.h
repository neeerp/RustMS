#pragma once

#include "../UIElement.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Text.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// An NPC's choice of styles (NPC dialogue types 7 and 8; v83 CUtilDlgEx mode 5, FUN_009a92c9 and
	// FUN_009aa708 on UIWindow.img/UtilDlgEx_Avatar). It shows the NPC and its question over the player's own
	// look wearing one face, hairstyle or skin at a time, by the first style's id / 10000: 2 faces, 3
	// hairstyles, others skins. Prev and Next go round the styles, TAKE OFF ALL shows the look without
	// equipment and DEFAULT SETTINGS brings it back. It is modal: OK or Enter answers with the style shown;
	// Cancel, the corner button or Escape answers with none.
	class UIAvatarSelect : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::AVATARSELECT;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `styles` must not be empty.
		UIAvatarSelect(int32_t npcid, int8_t msgtype, const std::string &text, std::vector<int32_t> styles);

		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum class Kind
		{
			SKIN,
			FACE,
			HAIR
		};

		// Show the style `index` steps round from the first.
		void show(size_t index);

		// Answer with the style shown, or with none.
		void answer(bool ok);

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			EXIT,
			PREV,
			NEXT,
			OFF,
			ON
		};

		int8_t msgtype;
		std::vector<int32_t> styles;
		Kind kind;
		size_t shown;
		// TAKE OFF ALL is in effect.
		bool bare;
		CharLook look;
		Texture speaker;
		Texture shadow;
		Texture name_tag;
		Text prompt;
		Text name;
	};
}
