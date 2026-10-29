#include "AriantHandlers.h"

#include "../../Character/Ariant.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIAriant.h"

namespace ms
{
	void AriantScoreHandler::handle(InPacket& recv) const
	{
		std::vector<ariant::Score> scores = ariant::read_scores(recv);

		if (!ariant::on_field())
			return;

		const CharStats& stats = Stage::get().get_player().get_stats();
		ariant::board().apply(scores, stats.get_name(), ariant::gm_job(stats.get_job().get_id()));
	}

	void AriantResultHandler::handle(InPacket&) const
	{
		if (!ariant::on_field())
			return;

		if (auto hud = UI::get().get_element<UIAriant>())
			hud->show_result();
	}
}
