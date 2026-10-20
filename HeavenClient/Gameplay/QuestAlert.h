#pragma once

#include "QuestAlertRules.h"

#include "../Graphics/Animation.h"
#include "../Template/Point.h"

namespace ms
{
	struct QuestInfo;

	// The quest alert: a bulb over the player's face while an autoStart quest can be started, an autoComplete one
	// completed, or an autoPreComplete one is under way. Clicking it lists them wherever the player is (report
	// 201-quest-alert).
	class QuestAlert
	{
	public:
		void init();

		// A new character: nothing is offered until it enters a field.
		void reset();

		// The player entered a field (FUN_00a086d5): the bulb starts over with the quests found again.
		void enter_field(int32_t mapid);

		// The player's stats changed (FUN_00a0843c): the normalAutoStart quests are checked again, with the alert.
		void stats_changed();

		// A quest's record changed to 0 forfeited, 1 started or 2 completed (FUN_00a20f4c).
		void record_changed(int16_t quest_id, int8_t record);

		// The checks every 10 s, and the bulb's frame (FUN_0094a144).
		void update();

		void draw(Point<int16_t> face, float alpha) const;

		// A press arms the bulb under the cursor, and the release on it lists its quests (FUN_0094fa8e). `face` is
		// on the screen.
		void send_cursor(Point<int16_t> face, Point<int16_t> cursor, bool pressed, bool held);

		void release_cursor();

	private:
		void offer(const QuestInfo &quest, bool notify);

		void check_complete(const QuestInfo &quest);

		void open_menu();

		const Animation &shown() const;

		quest_alert::State state;
		bool active = false;
		int32_t mapid = 0;
		// Ms since the character entered its first field, and since the autoComplete quests were last checked.
		int32_t clock = 0;
		int32_t since_check = 0;
		bool armed = false;
		// Effect/BasicEff.img/QuestAlert: Appear plays once, then Default loops.
		Animation appear;
		Animation idle;
	};
}
